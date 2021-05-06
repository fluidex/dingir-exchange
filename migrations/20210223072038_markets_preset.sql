-- Add migration script here

insert into asset (symbol, name, token_address, is_commonly_quoted, precision_stor, precision_show) 
	values ('ETH', 'Ether', '0x0000000000000000000000000000000000000000', true, 6, 6);
insert into asset (symbol, name, token_address, is_commonly_quoted, precision_stor, precision_show) 
	values ('UNI', 'Uniswap', '0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984', false, 6, 6);
insert into asset (symbol, name, token_address, is_commonly_quoted, precision_stor, precision_show) 
	values ('USDT', 'Tether USD', '0xdAC17F958D2ee523a2206206994597C13D831ec7', true, 6, 6);

insert into market (base_asset, quote_asset, precision_base, precision_quote, precision_fee, min_amount) values 
('0x0000000000000000000000000000000000000000', '0xdAC17F958D2ee523a2206206994597C13D831ec7', 4, 2, 2, 0.001)
,('0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984', '0xdAC17F958D2ee523a2206206994597C13D831ec7', 4, 2, 2, 0.001)
;
