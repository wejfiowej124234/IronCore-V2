-- ============================================================================
-- Migration: 0042_make_cross_chain_source_tx_hash_nullable.sql
-- Description: Allow non-custodial bridge execute to create a DB row before broadcast
--              by making source_tx_hash nullable.
-- ============================================================================

ALTER TABLE cross_chain_transactions
    ALTER COLUMN source_tx_hash DROP NOT NULL;
