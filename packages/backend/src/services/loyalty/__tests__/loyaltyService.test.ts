import { LoyaltyService } from '../loyaltyService';
import { LoyaltyTier } from '../../../types/loyalty';

describe('LoyaltyService Idempotent Tier Recalculation', () => {
  it('settles to a stable tier regardless of run order or point additions/subtractions', () => {
    const service = new LoyaltyService();
    const wallet = 'GTESTWALLET1234567890';

    service.addPoints(wallet, 15000);
    expect(service.getTier(wallet)).toBe(LoyaltyTier.GOLD);

    // Idempotent re-evaluation
    const account = (service as any).getOrCreateAccount(wallet);
    const tier1 = service.recomputeTierIdempotently(account);
    const tier2 = service.recomputeTierIdempotently(account);
    expect(tier1).toBe(LoyaltyTier.GOLD);
    expect(tier2).toBe(LoyaltyTier.GOLD);

    // Decreasing points drops tier correctly and idempotently
    service.redeemPoints(wallet, 14500);
    expect(service.getTier(wallet)).toBe(LoyaltyTier.BRONZE);
    service.recomputeTierIdempotently(account);
    expect(service.getTier(wallet)).toBe(LoyaltyTier.BRONZE);
  });
});
