-- Insert fee collector addresses for each chain
INSERT INTO gas.fee_collector_addresses (chain, address, active)
SELECT v.chain, v.address, v.active
FROM (
	VALUES
		('ethereum', '0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb', true),
		('bsc',      '0x8894E0a0c962CB723c1976a4421c95949bE2D4E3', true),
		('polygon',  '0x9965507D1a55bcC2695C58ba16FB37d819B0A4dc', true)
) AS v(chain, address, active)
WHERE NOT EXISTS (
	SELECT 1
	FROM gas.fee_collector_addresses a
	WHERE a.chain = v.chain
	  AND a.address = v.address
);
