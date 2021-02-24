-- Add migration script here

insert into asset (asset_name, precision_stor, precision_show) values ('ETH', 6, 6);
insert into asset (asset_name, precision_stor, precision_show) values ('USDT', 6, 6);

insert into market 
(base_asset, quote_asset, precision_base, precision_quote, precision_fee, min_amount) 
values ('ETH', 'USDT', 4, 2, 2, 0.001);
