-- Add migration script here

insert into asset (id, symbol, name, token_address, rollup_token_id, precision_stor, precision_show) values 
	('ETH', 'ETH', 'Ether', '', 0, 6, 6),
	('USDT', 'USDT', 'Tether USD', '0xdAC17F958D2ee523a2206206994597C13D831ec7', 1, 6, 6),
	('UNI', 'UNI', 'Uniswap', '0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984', 2, 6, 6),
	('LINK', 'LINK', 'ChainLink Token', '0x514910771af9ca656af840dff83e8264ecf986ca', 3, 6, 6),
	('YFI', 'YFI', 'yearn.finance', '0x0bc529c00C6401aEF6D220BE8C6Ea1667F6Ad93e', 4, 6, 6),
	('MATIC', 'MATIC', 'Matic Token', '0x7d1afa7b718fb893db30a3abc0cfc608aacfebb0', 5, 6, 6)
	;

-- Fee is disabled
insert into market (base_asset, quote_asset, precision_amount, precision_price, precision_fee, min_amount) values 
	('ETH', 'USDT', 4, 2, 0, 0.001),
	('UNI', 'USDT', 4, 2, 0, 0.001),
	('LINK', 'USDT', 4, 2, 0, 0.001),
	('MATIC', 'USDT', 4, 2, 0, 0.001)
	;
