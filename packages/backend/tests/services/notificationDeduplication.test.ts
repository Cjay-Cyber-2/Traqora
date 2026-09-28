import { describe, it, expect, beforeEach, jest } from "@jest/globals";
import { setupNotificationWorker } from "../../packages/backend/src/jobs/notificationWorker";
import { AppDataSource } from "../../packages/backend/src/db/dataSource";
import { NotificationLog } from "../../packages/backend/src/db/entities/NotificationLog";
import { UserPreference } from "../../packages/backend/src/db/entities/UserPreference";

const sendTemplateMock = jest.fn().mockResolvedValue(true);
const sendSMSMock = jest.fn().mockResolvedValue({ id: "sms-1" });
const sendPushMock = jest.fn().mockResolvedValue({ successful: 1, failed: 0 });

jest.mock("../../packages/backend/src/services/EmailService", () => ({
  emailService: { sendTemplate: (...args: any[]) => sendTemplateMock(...args) },
}));
jest.mock("../../packages/backend/src/services/SMSService", () => ({
  smsService: { sendSMS: (...args: any[]) => sendSMSMock(...args) },
}));
jest.mock("../../packages/backend/src/services/PushNotificationService", () => ({
  pushNotificationService: { sendPush: (...args: any[]) => sendPushMock(...args) },
}));

const processHandlers: Record<string, any> = {};
jest.mock("../../packages/backend/src/jobs/notificationQueue", () => ({
  notificationQueue: {
    process: jest.fn((cb) => {
      processHandlers["process"] = cb;
    }),
  },
}));

describe("Multi-channel notification de-duplication", () => {
  beforeEach(async () => {
    jest.clearAllMocks();
    if (!AppDataSource.isInitialized) {
      await AppDataSource.initialize();
    }
    await AppDataSource.getRepository(NotificationLog).clear();
    await AppDataSource.getRepository(UserPreference).clear();
    setupNotificationWorker();
  });

  it("should prevent duplicate notification sends across channels for the same event payload using NotificationLog", async () => {
    const userId = "user-dedup-1";
    const userPrefRepo = AppDataSource.getRepository(UserPreference);
    await userPrefRepo.save(
      userPrefRepo.create({
        userId,
        emailEnabled: true,
        smsEnabled: true,
        pushEnabled: true,
        email: "test@example.com",
        phoneNumber: "+15555550199",
        fcmToken: "token123",
      })
    );

    const jobData = {
      userId,
      type: "booking_confirmed",
      data: { bookingId: "bk-999" },
      channels: ["email", "sms", "push"],
    };

    const processor = processHandlers["process"];
    expect(processor).toBeDefined();

    // First invocation should succeed and log sent entries
    const res1 = await processor({ data: jobData, attemptsMade: 0, opts: { attempts: 3 } });
    expect(sendTemplateMock).toHaveBeenCalledTimes(1);
    expect(sendSMSMock).toHaveBeenCalledTimes(1);
    expect(sendPushMock).toHaveBeenCalledTimes(1);

    // Second invocation with identical payload should be de-duplicated and skipped
    const res2 = await processor({ data: jobData, attemptsMade: 0, opts: { attempts: 3 } });
    expect(sendTemplateMock).toHaveBeenCalledTimes(1); // Call count should not increase
    expect(sendSMSMock).toHaveBeenCalledTimes(1);
    expect(sendPushMock).toHaveBeenCalledTimes(1);
  });
});
