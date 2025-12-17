-- Fix invalid-length Ethereum fee collector address inserted by older seed/migration
-- Ethereum address must be 0x + 40 hex chars.

UPDATE gas.fee_collector_addresses
SET address = '0x742d35cc6634c0532925a3b844bc9e7595f0beb6'
WHERE chain = 'ethereum'
  AND address = '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb';
