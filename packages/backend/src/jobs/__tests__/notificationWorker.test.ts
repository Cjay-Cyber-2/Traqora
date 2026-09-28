import { setupNotificationWorker } from '../notificationWorker';
import { notificationQueue } from '../notificationQueue';
import { AppDataSource } from '../../db/dataSource';
import { NotificationLog } from '../../db/entities/NotificationLog';
import { UserPreference } from '../../db/entities/UserPreference';
import { emailService } from '../../services/EmailService';
import { smsService } from '../../services/SMSService';
import { pushNotificationService } from '../../services/PushNotificationService';

jest.mock('../notificationQueue', () => ({
  notificationQueue: {
    process: jest.fn(),
    on: jest.fn(),
  },
}));

jest.mock('../../db/dataSource', () => ({
  AppDataSource: {
    getRepository: jest.fn(),
  },
}));

jest.mock('../../services/EmailService', () => ({
  emailService: {
    sendTemplate: jest.fn(),
  },
}));

jest.mock('../../services/SMSService', () => ({
  smsService: {
    sendSMS: jest.fn(),
  },
}));

jest.mock('../../services/PushNotificationService', () => ({
  pushNotificationService: {
    sendPush: jest.fn(),
  },
}));

describe('Notification Worker De-duplication', () => {
  let processHandler: (job: any) => Promise<any>;

  beforeEach(() => {
    jest.clearAllMocks();
    (notificationQueue.process as any).mockImplementation((handler: any) => {
      processHandler = handler;
    });
    setupNotificationWorker();
  });

  it('should skip sending if a recent notification was already sent on any channel', async () => {
    const mockUserPrefRepo = {
      findOne: jest.fn().mockResolvedValue({
        userId: 'user-1',
        emailEnabled: true,
        email: 'test@example.com',
        smsEnabled: true,
        phoneNumber: '+1234567890',
        pushEnabled: true,
        fcmToken: 'token',
      }),
    };

    const mockLogRepo = {
      findOne: jest.fn().mockResolvedValue({
        id: 1,
        userId: 'user-1',
        channel: 'email',
        type: 'BOOKING_CONFIRMED',
        status: 'sent',
        createdAt: new Date(),
      }),
      create: jest.fn((val) => val),
      save: jest.fn(),
    };

    (AppDataSource.getRepository as any).mockImplementation((entity: any) => {
      if (entity === UserPreference) return mockUserPrefRepo;
      if (entity === NotificationLog) return mockLogRepo;
    });

    const job = {
      id: 'job-1',
      data: {
        userId: 'user-1',
        type: 'BOOKING_CONFIRMED',
        data: { bookingId: '123' },
        channels: ['email', 'sms', 'push'],
      },
      attemptsMade: 0,
    };

    const results = await processHandler!(job);

    expect(results).toContainEqual({ channel: 'email', status: 'skipped_duplicate' });
    expect(results).toContainEqual({ channel: 'sms', status: 'skipped_duplicate' });
    expect(results).toContainEqual({ channel: 'push', status: 'skipped_duplicate' });
    expect(emailService.sendTemplate).not.toHaveBeenCalled();
    expect(smsService.sendSMS).not.toHaveBeenCalled();
    expect(pushNotificationService.sendPush).not.toHaveBeenCalled();
  });
});
