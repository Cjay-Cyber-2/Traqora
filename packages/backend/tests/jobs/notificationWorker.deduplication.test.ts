import { setupNotificationWorker } from '../../src/jobs/notificationWorker';
import { notificationQueue } from '../../src/jobs/notificationQueue';
import { AppDataSource } from '../../src/db/dataSource';
import { UserPreference } from '../../src/db/entities/UserPreference';
import { NotificationLog } from '../../src/db/entities/NotificationLog';
import { emailService } from '../../src/services/EmailService';
import { smsService } from '../../src/services/SMSService';
import { pushNotificationService } from '../../src/services/PushNotificationService';

jest.mock('../../src/services/EmailService', () => ({
  emailService: { sendTemplate: jest.fn().mockResolvedValue(true) },
}));
jest.mock('../../src/services/SMSService', () => {
  const original = jest.requireActual('../../src/services/SMSService');
  return {
    ...original,
    smsService: { sendSMS: jest.fn().mockResolvedValue({ id: 'sms-1' }) },
  };
});
jest.mock('../../src/services/PushNotificationService', () => ({
  pushNotificationService: { sendPush: jest.fn().mockResolvedValue({ successful: 1, failed: 0 }) },
}));

let processHandler: (job: any) => Promise<any>;

jest.mock('../../src/jobs/notificationQueue', () => ({
  notificationQueue: {
    process: jest.fn((handler) => {
      processHandler = handler;
    }),
    on: jest.fn(),
  },
}));

describe('Notification Worker Multi-Channel De-duplication', () => {
  beforeAll(async () => {
    if (!AppDataSource.isInitialized) {
      await AppDataSource.initialize();
    }
    setupNotificationWorker();
  });

  afterEach(async () => {
    await AppDataSource.getRepository(NotificationLog).clear();
    await AppDataSource.getRepository(UserPreference).clear();
    jest.clearAllMocks();
  });

  afterAll(async () => {
    if (AppDataSource.isInitialized) {
      await AppDataSource.destroy();
    }
  });

  it('should skip duplicate notifications within the deduplication window', async () => {
    const userId = 'user-dedup-1';
    const prefRepo = AppDataSource.getRepository(UserPreference);
    const logRepo = AppDataSource.getRepository(NotificationLog);

    await prefRepo.save(
      prefRepo.create({
        userId,
        emailEnabled: true,
        email: 'test@example.com',
        smsEnabled: true,
        phoneNumber: '+1234567890',
        pushEnabled: true,
        fcmToken: 'token123',
      }),
    );

    // Pre-insert an existing sent log for email within the window
    await logRepo.save(
      logRepo.create({
        userId,
        channel: 'email',
        type: 'booking_confirmation',
        status: 'sent',
        createdAt: new Date(),
      }),
    );

    const job = {
      id: 'job-dedup-test',
      attemptsMade: 0,
      data: {
        userId,
        type: 'booking_confirmation',
        channels: ['email', 'sms'],
        data: { bookingId: 'b-1' },
      },
    };

    const result = await processHandler(job);

    // Email should be skipped as duplicate, SMS should be sent
    expect(emailService.sendTemplate).not.toHaveBeenCalled();
    expect(smsService.sendSMS).toHaveBeenCalled();
  });
});
