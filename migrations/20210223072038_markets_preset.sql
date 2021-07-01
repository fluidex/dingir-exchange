-- Add migration script here

insert into asset (id, symbol, name, token_address, rollup_token_id, precision_stor, precision_show) values 
	('ETH', 'ETH', 'Ether', '', 0, 6, 6),
	('USDT', 'USDT', 'Tether USD', '0xdAC17F958D2ee523a2206206994597C13D831ec7', 1, 6, 6),
	('UNI', 'UNI', 'Uniswap', '0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984', 2, 6, 6),
	('LINK', 'LINK', 'LINK', '', 3, 6, 6),
	('YFI', 'YFI', 'YFI', '', 4, 6, 6),
	('MATIC', 'MATIC', 'MATIC', '', 5, 6, 6)
	;

-- Fee is disabled
insert into market (base_asset, quote_asset, precision_amount, precision_price, precision_fee, min_amount) values 
	('ETH', 'USDT', 4, 2, 0, 0.001),
	('UNI', 'USDT', 4, 2, 0, 0.001),
	('LINK', 'USDT', 4, 2, 0, 0.001),
	('MATIC', 'USDT', 4, 2, 0, 0.001)
	;
