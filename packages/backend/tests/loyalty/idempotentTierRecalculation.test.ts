import { calculateTargetTier, recalculateLoyaltyTier, calculateTierProgression } from '../../src/services/loyalty-tiers';
import { LoyaltyTier, LoyaltyAccount } from '../../src/types/loyalty';

function makeAccount(overrides: Partial<LoyaltyAccount> = {}): LoyaltyAccount {
  return {
    userId: 'user-idempotency',
    tier: LoyaltyTier.BRONZE,
    totalPoints: 0,
    availablePoints: 0,
    lifetimeBookings: 0,
    lifetimeSpent: 0,
    tierUpdatedAt: new Date(),
    createdAt: new Date(),
    updatedAt: new Date(),
    ...overrides,
  };
}

describe('Idempotent Loyalty Tier Recalculation (#512 backend equivalent)', () => {
  it('calculates the correct target tier based strictly on total points thresholds', () => {
    expect(calculateTargetTier(0)).toBe(LoyaltyTier.BRONZE);
    expect(calculateTargetTier(999)).toBe(LoyaltyTier.BRONZE);
    expect(calculateTargetTier(1000)).toBe(LoyaltyTier.SILVER);
    expect(calculateTargetTier(9999)).toBe(LoyaltyTier.SILVER);
    expect(calculateTargetTier(10000)).toBe(LoyaltyTier.GOLD);
    expect(calculateTargetTier(50000)).toBe(LoyaltyTier.PLATINUM);
    expect(calculateTargetTier(200000)).toBe(LoyaltyTier.DIAMOND);
    expect(calculateTargetTier(500000)).toBe(LoyaltyTier.DIAMOND);
  });

  it('is fully idempotent and independent of run order', () => {
    const account = makeAccount({ totalPoints: 15000, tier: LoyaltyTier.BRONZE });

    const firstRun = recalculateLoyaltyTier(account);
    expect(firstRun.tier).toBe(LoyaltyTier.GOLD);
    expect(firstRun.changed).toBe(true);

    account.tier = firstRun.tier;

    const secondRun = recalculateLoyaltyTier(account);
    expect(secondRun.tier).toBe(LoyaltyTier.GOLD);
    expect(secondRun.changed).toBe(false);

    const thirdRun = recalculateLoyaltyTier(account);
    expect(thirdRun.tier).toBe(LoyaltyTier.GOLD);
    expect(thirdRun.changed).toBe(false);
  });

  it('settles to a stable state regardless of initial stale tier configuration', () => {
    const staleHighAccount = makeAccount({ totalPoints: 500, tier: LoyaltyTier.DIAMOND });
    const res1 = recalculateLoyaltyTier(staleHighAccount);
    expect(res1.tier).toBe(LoyaltyTier.BRONZE);
    expect(res1.changed).toBe(true);

    staleHighAccount.tier = res1.tier;
    const res2 = recalculateLoyaltyTier(staleHighAccount);
    expect(res2.tier).toBe(LoyaltyTier.BRONZE);
    expect(res2.changed).toBe(false);
  });

  it('handles progression correctly and stably at boundaries', () => {
    const account = makeAccount({ totalPoints: 10000, tier: LoyaltyTier.GOLD });
    const progression = calculateTierProgression(account);
    expect(progression.currentTier.tier).toBe(LoyaltyTier.GOLD);
    expect(progression.nextTier?.tier).toBe(LoyaltyTier.PLATINUM);
    expect(progression.pointsRemaining).toBe(40000);
    expect(progression.progressPercent).toBe(0);
  });
});
