-- Fix invalid-length Ethereum fee collector address inserted by older seed/migration
-- Ethereum address must be 0x + 40 hex chars.

UPDATE gas.fee_collector_addresses
SET address = '0xf5c533a89fb01a1e8e1c288433a345a713027244'
WHERE chain = 'ethereum'
  AND address = '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb';
