import { v4 as uuidv4 } from 'uuid';

interface BookingRequest {
  supplierReference?: string;
  offerId: string;
  userId: string;
}

interface BookingResult {
  id: string;
  createdAt: string;
  offerId: string;
  userId: string;
  supplierReference?: string;
}

const processedBookings = new Map<string, BookingResult>();

export async function createBooking(request: BookingRequest): Promise<BookingResult> {
  const key = request.supplierReference ? `ref-${request.supplierReference}` : `auto-${uuidv4()}`;

  if (request.supplierReference && processedBookings.has(key)) {
    return processedBookings.get(key)!;
  }

  const result: BookingResult = {
    id: uuidv4(),
    createdAt: new Date().toISOString(),
    offerId: request.offerId,
    userId: request.userId,
    supplierReference: request.supplierReference,
  };

  if (request.supplierReference) {
    processedBookings.set(key, result);
  }
  return result;
}
