-- Add migration script here

insert into asset (id, symbol, name, token_address, precision_stor, precision_show) 
	values ('ETH', 'ETH', 'Ether', '', 6, 6);
insert into asset (id, symbol, name, token_address, precision_stor, precision_show) 
	values ('UNI', 'UNI', 'Uniswap', '0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984', 6, 6);
insert into asset (id, symbol, name, token_address, precision_stor, precision_show) 
	values ('USDT', 'USDT', 'Tether USD', '0xdAC17F958D2ee523a2206206994597C13D831ec7', 6, 6);

insert into market (base_asset, quote_asset, precision_base, precision_quote, precision_fee, min_amount) values 
('ETH', 'USDT', 4, 2, 2, 0.001)
,('UNI', 'USDT', 4, 2, 2, 0.001)
;
