import { LoyaltyTier } from '../../types/loyalty';
import { TIERS } from '../loyalty-tiers';

export interface LoyaltyAccountState {
  walletAddress: string;
  points: number;
  tier: LoyaltyTier;
  tierUpdatedAt?: Date;
}

export class LoyaltyService {
  private accounts = new Map<string, LoyaltyAccountState>();

  getBalance(walletAddress: string): number {
    return this.accounts.get(walletAddress)?.points || 0;
  }

  addPoints(walletAddress: string, points: number): number {
    const account = this.getOrCreateAccount(walletAddress);
    account.points += points;
    this.recomputeTierIdempotently(account);
    return account.points;
  }

  redeemPoints(walletAddress: string, points: number): number {
    const account = this.getOrCreateAccount(walletAddress);
    if (account.points < points) {
      throw new Error('Insufficient points');
    }
    account.points -= points;
    this.recomputeTierIdempotently(account);
    return account.points;
  }

  setTier(walletAddress: string, tier: LoyaltyTier): LoyaltyTier {
    const account = this.getOrCreateAccount(walletAddress);
    account.tier = tier;
    account.tierUpdatedAt = new Date();
    return tier;
  }

  getTier(walletAddress: string): LoyaltyTier | undefined {
    return this.accounts.get(walletAddress)?.tier;
  }

  recomputeTierIdempotently(account: LoyaltyAccountState): LoyaltyTier {
    let targetTier = TIERS[0].tier;
    for (const t of TIERS) {
      if (account.points >= t.minPoints) {
        targetTier = t.tier;
      }
    }
    if (account.tier !== targetTier) {
      account.tier = targetTier;
      account.tierUpdatedAt = new Date();
    }
    return account.tier;
  }

  private getOrCreateAccount(walletAddress: string): LoyaltyAccountState {
    let account = this.accounts.get(walletAddress);
    if (!account) {
      account = {
        walletAddress,
        points: 0,
        tier: LoyaltyTier.BRONZE,
      };
      this.accounts.set(walletAddress, account);
    }
    return account;
  }
}

export const loyaltyService = new LoyaltyService();
