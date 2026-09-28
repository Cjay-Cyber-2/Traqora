import { describe, it, expect, beforeEach, jest } from "@jest/globals";
import { NotificationService } from "../NotificationService";
import type { NotificationChannel, NotificationCategory } from "../../types/notification";

describe("NotificationService Multi-channel De-duplication", () => {
  let service: NotificationService;

  beforeEach(() => {
    service = new NotificationService();
  });

  it("should detect duplicate notification within the time window", async () => {
    const userId = "user-dedup";
    const channel = "email";
    const type = "booking_confirmation";
    const payload = { bookingId: "b-123" };

    // Manually push delivery log simulating a sent notification
    const logs = (service as any).deliveryLogs.get(userId) || [];
    logs.push({
      id: "log-1",
      userId,
      channel,
      type,
      payload,
      status: "sent",
      timestamp: new Date(),
      attempts: 1,
    });
    (service as any).deliveryLogs.set(userId, logs);

    const isDup = await service.isDuplicateNotification(userId, channel, type, payload, 60_000);
    expect(isDup).toBe(true);
  });

  it("should not flag different notification type as duplicate", async () => {
    const userId = "user-dedup";
    const channel = "email";
    const payload = { bookingId: "b-123" };

    const logs = (service as any).deliveryLogs.get(userId) || [];
    logs.push({
      id: "log-1",
      userId,
      channel,
      type: "booking_confirmation",
      payload,
      status: "sent",
      timestamp: new Date(),
      attempts: 1,
    });
    (service as any).deliveryLogs.set(userId, logs);

    const isDup = await service.isDuplicateNotification(userId, channel, "flight_delayed", payload, 60_000);
    expect(isDup).toBe(false);
  });
});
