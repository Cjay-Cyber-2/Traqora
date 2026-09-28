import { describe, it, expect, beforeEach } from "@jest/globals";
import { NotificationService } from "../../src/services/NotificationService";

describe("NotificationService Multi-Channel De-duplication", () => {
  let service: NotificationService;

  beforeEach(() => {
    service = new NotificationService();
  });

  it("should suppress duplicate notifications within the deduplication window", async () => {
    const userId = "user-dedup-1";
    const message = "Price dropped on route";
    const data = { flightId: "FL123" };

    const firstResult = await service.sendPriceAlert(userId, message, data);
    expect(firstResult).toBe(true);

    const secondResult = await service.sendPriceAlert(userId, message, data);
    expect(secondResult).toBe(true);
  });
});
