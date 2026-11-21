import { createBooking } from '../../src/services/bookingService';

describe('Booking Idempotency', () => {
  it('returns the same booking record when supplierReference is duplicated', async () => {
    const ref = 'supp-ref-12345';
    const first = await createBooking({ supplierReference: ref, offerId: 'offer-1', userId: 'user-1' });
    const second = await createBooking({ supplierReference: ref, offerId: 'offer-1', userId: 'user-1' });

    expect(first.id).toEqual(second.id);
    expect(first.createdAt).toEqual(second.createdAt);
  });

  it('creates distinct bookings when supplierReference is absent or different', async () => {
    const first = await createBooking({ offerId: 'offer-2', userId: 'user-1' });
    const second = await createBooking({ offerId: 'offer-2', userId: 'user-1' });

    expect(first.id).not.toEqual(second.id);
  });
});
