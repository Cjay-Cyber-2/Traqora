import { describe, it, expect, beforeEach } from "@jest/globals";
import { NotificationService } from "../../src/services/NotificationService";
import type { NotificationChannel } from "../../src/types/notification";

describe("NotificationService Multi-Channel De-duplication", () => {
  let service: NotificationService;

  beforeEach(() => {
    service = new NotificationService();
  });

  it("should detect duplicate notification within deduplication window", async () => {
    const userId = "user-dedup-1";
    const channel: NotificationChannel = "email";
    const type = "booking_confirmation";

    const initialCheck = await service.isDuplicateNotification(userId, channel, type);
    expect(initialCheck).toBe(false);

    await service.recordDeliveryLog(userId, channel, type, "sent");

    const subsequentCheck = await service.isDuplicateNotification(userId, channel, type);
    expect(subsequentCheck).toBe(true);
  });

  it("should allow notifications on different channels", async () => {
    const userId = "user-dedup-2";
    const type = "booking_confirmation";

    await service.recordDeliveryLog(userId, "email", type, "sent");

    expect(await service.isDuplicateNotification(userId, "email", type)).toBe(true);
    expect(await service.isDuplicateNotification(userId, "sms", type)).toBe(false);
  });
});
