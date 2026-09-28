import { recalculateTierIdempotent, TIERS } from './loyalty-tiers';
import { LoyaltyTier, LoyaltyAccount } from '../types/loyalty';

describe('recalculateTierIdempotent', () => {
  const baseAccount: LoyaltyAccount = {
    id: 'acc-1',
    userId: 'user-1',
    tier: LoyaltyTier.BRONZE,
    totalPoints: 500,
    availablePoints: 500,
    lifetimePoints: 500,
    createdAt: new Date(),
    updatedAt: new Date(),
  };

  it('should be idempotent and return no change when tier matches totalPoints', () => {
    const account = { ...baseAccount, totalPoints: 1500, tier: LoyaltyTier.SILVER };
    const first = recalculateTierIdempotent(account);
    const second = recalculateTierIdempotent({ ...account, tier: first.updatedTier });
    
    expect(first.updatedTier).toBe(LoyaltyTier.SILVER);
    expect(first.changed).toBe(false);
    expect(second.changed).toBe(false);
    expect(second.updatedTier).toBe(first.updatedTier);
  });

  it('should transition correctly and settle to a stable state independently of run order', () => {
    const account: LoyaltyAccount = { ...baseAccount, totalPoints: 15000, tier: LoyaltyTier.BRONZE };
    
    const res1 = recalculateTierIdempotent(account);
    expect(res1.updatedTier).toBe(LoyaltyTier.GOLD);
    expect(res1.changed).toBe(true);

    const res2 = recalculateTierIdempotent({ ...account, tier: res1.updatedTier });
    expect(res2.updatedTier).toBe(LoyaltyTier.GOLD);
    expect(res2.changed).toBe(false);
  });
});
