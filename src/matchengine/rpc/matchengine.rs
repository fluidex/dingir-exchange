///
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct UserInfo {
    #[prost(uint32, tag = "1")]
    pub user_id: u32,
    #[prost(string, tag = "2")]
    pub l1_address: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub l2_pubkey: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct BalanceQueryRequest {
    #[prost(uint32, tag = "1")]
    pub user_id: u32,
    /// optional
    #[prost(string, repeated, tag = "2")]
    pub assets: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct BalanceQueryResponse {
    #[prost(message, repeated, tag = "1")]
    pub balances: ::prost::alloc::vec::Vec<balance_query_response::AssetBalance>,
}
/// Nested message and enum types in `BalanceQueryResponse`.
pub mod balance_query_response {
    #[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
    pub struct AssetBalance {
        #[prost(string, tag = "1")]
        pub asset_id: ::prost::alloc::string::String,
        #[prost(string, tag = "2")]
        pub available: ::prost::alloc::string::String,
        #[prost(string, tag = "3")]
        pub frozen: ::prost::alloc::string::String,
    }
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct BalanceUpdateRequest {
    #[prost(uint32, tag = "1")]
    pub user_id: u32,
    #[prost(string, tag = "2")]
    pub asset: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub business: ::prost::alloc::string::String,
    #[prost(uint64, tag = "4")]
    pub business_id: u64,
    #[prost(string, tag = "5")]
    pub delta: ::prost::alloc::string::String,
    #[prost(string, tag = "6")]
    pub detail: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct BalanceUpdateResponse {}
/// repeated string assets = 1;
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct AssetListRequest {}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct AssetListResponse {
    #[prost(message, repeated, tag = "1")]
    pub asset_lists: ::prost::alloc::vec::Vec<asset_list_response::AssetInfo>,
}
/// Nested message and enum types in `AssetListResponse`.
pub mod asset_list_response {
    #[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
    pub struct AssetInfo {
        #[prost(string, tag = "1")]
        pub symbol: ::prost::alloc::string::String,
        #[prost(string, tag = "2")]
        pub name: ::prost::alloc::string::String,
        #[prost(int32, tag = "3")]
        pub chain_id: i32,
        #[prost(string, tag = "4")]
        pub token_address: ::prost::alloc::string::String,
        #[prost(uint32, tag = "5")]
        pub precision: u32,
        #[prost(string, tag = "6")]
        pub logo_uri: ::prost::alloc::string::String,
        #[prost(int32, tag = "7")]
        pub inner_id: i32,
    }
}
///
/// internal?
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct AssetSummaryRequest {
    #[prost(string, repeated, tag = "1")]
    pub assets: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct AssetSummaryResponse {
    #[prost(message, repeated, tag = "1")]
    pub asset_summaries: ::prost::alloc::vec::Vec<asset_summary_response::AssetSummaryInfo>,
}
/// Nested message and enum types in `AssetSummaryResponse`.
pub mod asset_summary_response {
    #[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
    pub struct AssetSummaryInfo {
        #[prost(string, tag = "1")]
        pub name: ::prost::alloc::string::String,
        #[prost(string, tag = "2")]
        pub total_balance: ::prost::alloc::string::String,
        #[prost(int32, tag = "3")]
        pub available_count: i32,
        #[prost(string, tag = "4")]
        pub available_balance: ::prost::alloc::string::String,
        #[prost(int32, tag = "5")]
        pub frozen_count: i32,
        #[prost(string, tag = "6")]
        pub frozen_balance: ::prost::alloc::string::String,
    }
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct OrderPutRequest {
    #[prost(uint32, tag = "1")]
    pub user_id: u32,
    #[prost(string, tag = "2")]
    pub market: ::prost::alloc::string::String,
    #[prost(enumeration = "OrderSide", tag = "3")]
    pub order_side: i32,
    #[prost(enumeration = "OrderType", tag = "4")]
    pub order_type: i32,
    /// always amount for base, even for market bid
    #[prost(string, tag = "5")]
    pub amount: ::prost::alloc::string::String,
    /// should be empty or zero for market order
    #[prost(string, tag = "6")]
    pub price: ::prost::alloc::string::String,
    /// onyl valid for market bid order
    #[prost(string, tag = "7")]
    pub quote_limit: ::prost::alloc::string::String,
    #[prost(string, tag = "8")]
    pub taker_fee: ::prost::alloc::string::String,
    #[prost(string, tag = "9")]
    pub maker_fee: ::prost::alloc::string::String,
    /// Ensures an Limit order is only subject to Maker Fees (ignored for Market orders).
    #[prost(bool, tag = "10")]
    pub post_only: bool,
    /// bjj signature used in Fluidex
    #[prost(string, tag = "11")]
    pub signature: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct OrderInfo {
    #[prost(uint64, tag = "1")]
    pub id: u64,
    #[prost(string, tag = "2")]
    pub market: ::prost::alloc::string::String,
    #[prost(enumeration = "OrderSide", tag = "3")]
    pub order_side: i32,
    #[prost(enumeration = "OrderType", tag = "4")]
    pub order_type: i32,
    #[prost(uint32, tag = "5")]
    pub user_id: u32,
    #[prost(double, tag = "6")]
    pub create_time: f64,
    #[prost(double, tag = "7")]
    pub update_time: f64,
    #[prost(string, tag = "8")]
    pub price: ::prost::alloc::string::String,
    #[prost(string, tag = "9")]
    pub amount: ::prost::alloc::string::String,
    #[prost(string, tag = "10")]
    pub taker_fee: ::prost::alloc::string::String,
    #[prost(string, tag = "11")]
    pub maker_fee: ::prost::alloc::string::String,
    #[prost(string, tag = "12")]
    pub remain: ::prost::alloc::string::String,
    #[prost(string, tag = "13")]
    pub finished_base: ::prost::alloc::string::String,
    #[prost(string, tag = "14")]
    pub finished_quote: ::prost::alloc::string::String,
    #[prost(string, tag = "15")]
    pub finished_fee: ::prost::alloc::string::String,
    #[prost(bool, tag = "16")]
    pub post_only: bool,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct OrderQueryRequest {
    #[prost(uint32, tag = "1")]
    pub user_id: u32,
    #[prost(string, tag = "2")]
    pub market: ::prost::alloc::string::String,
    #[prost(int32, tag = "3")]
    pub offset: i32,
    #[prost(int32, tag = "4")]
    pub limit: i32,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct OrderQueryResponse {
    #[prost(int32, tag = "1")]
    pub offset: i32,
    #[prost(int32, tag = "2")]
    pub limit: i32,
    #[prost(int32, tag = "3")]
    pub total: i32,
    #[prost(message, repeated, tag = "4")]
    pub orders: ::prost::alloc::vec::Vec<OrderInfo>,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct OrderCancelRequest {
    #[prost(uint32, tag = "1")]
    pub user_id: u32,
    #[prost(string, tag = "2")]
    pub market: ::prost::alloc::string::String,
    #[prost(uint64, tag = "3")]
    pub order_id: u64,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct OrderCancelAllRequest {
    #[prost(uint32, tag = "1")]
    pub user_id: u32,
    #[prost(string, tag = "2")]
    pub market: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct OrderCancelAllResponse {
    #[prost(uint32, tag = "1")]
    pub total: u32,
}
/// why not both side
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct OrderBookRequest {
    #[prost(string, tag = "1")]
    pub market: ::prost::alloc::string::String,
    #[prost(enumeration = "OrderSide", tag = "2")]
    pub side: i32,
    #[prost(int32, tag = "3")]
    pub offset: i32,
    #[prost(int32, tag = "4")]
    pub limit: i32,
}
/// strange api
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct OrderBookResponse {
    #[prost(int32, tag = "1")]
    pub offset: i32,
    #[prost(int32, tag = "2")]
    pub limit: i32,
    #[prost(uint64, tag = "3")]
    pub total: u64,
    #[prost(message, repeated, tag = "4")]
    pub orders: ::prost::alloc::vec::Vec<OrderInfo>,
}
/// with cache
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct OrderBookDepthRequest {
    #[prost(string, tag = "1")]
    pub market: ::prost::alloc::string::String,
    #[prost(int32, tag = "2")]
    pub limit: i32,
    #[prost(string, tag = "3")]
    pub interval: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct OrderBookDepthResponse {
    #[prost(message, repeated, tag = "1")]
    pub asks: ::prost::alloc::vec::Vec<order_book_depth_response::PriceInfo>,
    #[prost(message, repeated, tag = "2")]
    pub bids: ::prost::alloc::vec::Vec<order_book_depth_response::PriceInfo>,
}
/// Nested message and enum types in `OrderBookDepthResponse`.
pub mod order_book_depth_response {
    #[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
    pub struct PriceInfo {
        #[prost(string, tag = "1")]
        pub price: ::prost::alloc::string::String,
        #[prost(string, tag = "2")]
        pub amount: ::prost::alloc::string::String,
    }
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct OrderDetailRequest {
    #[prost(string, tag = "1")]
    pub market: ::prost::alloc::string::String,
    #[prost(uint64, tag = "2")]
    pub order_id: u64,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct MarketListRequest {}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct MarketListResponse {
    #[prost(message, repeated, tag = "1")]
    pub markets: ::prost::alloc::vec::Vec<market_list_response::MarketInfo>,
}
/// Nested message and enum types in `MarketListResponse`.
pub mod market_list_response {
    #[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
    pub struct MarketInfo {
        #[prost(string, tag = "1")]
        pub name: ::prost::alloc::string::String,
        /// base
        #[prost(string, tag = "2")]
        pub base: ::prost::alloc::string::String,
        /// quote
        #[prost(string, tag = "3")]
        pub quote: ::prost::alloc::string::String,
        #[prost(uint32, tag = "4")]
        pub fee_precision: u32,
        #[prost(uint32, tag = "5")]
        pub amount_precision: u32,
        #[prost(uint32, tag = "6")]
        pub price_precision: u32,
        #[prost(string, tag = "7")]
        pub min_amount: ::prost::alloc::string::String,
    }
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct MarketSummaryRequest {
    #[prost(string, repeated, tag = "1")]
    pub markets: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct MarketSummaryResponse {
    #[prost(message, repeated, tag = "1")]
    pub market_summaries: ::prost::alloc::vec::Vec<market_summary_response::MarketSummary>,
}
/// Nested message and enum types in `MarketSummaryResponse`.
pub mod market_summary_response {
    #[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
    pub struct MarketSummary {
        #[prost(string, tag = "1")]
        pub name: ::prost::alloc::string::String,
        #[prost(int32, tag = "2")]
        pub ask_count: i32,
        #[prost(string, tag = "3")]
        pub ask_amount: ::prost::alloc::string::String,
        #[prost(int32, tag = "4")]
        pub bid_count: i32,
        #[prost(string, tag = "5")]
        pub bid_amount: ::prost::alloc::string::String,
        #[prost(uint64, tag = "6")]
        pub trade_count: u64,
    }
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct ReloadMarketsRequest {
    #[prost(bool, tag = "1")]
    pub from_scratch: bool,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct SimpleSuccessResponse {}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct TransferRequest {
    /// user_id
    #[prost(uint32, tag = "1")]
    pub from: u32,
    /// user_id
    #[prost(uint32, tag = "2")]
    pub to: u32,
    #[prost(string, tag = "3")]
    pub asset: ::prost::alloc::string::String,
    /// should be > 0
    #[prost(string, tag = "4")]
    pub delta: ::prost::alloc::string::String,
    #[prost(string, tag = "5")]
    pub memo: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct TransferResponse {
    #[prost(bool, tag = "1")]
    pub success: bool,
    #[prost(string, tag = "2")]
    pub asset: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub balance_from: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct DebugDumpRequest {}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct DebugDumpResponse {}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct DebugResetRequest {}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct DebugResetResponse {}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct DebugReloadRequest {}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct DebugReloadResponse {}
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum OrderSide {
    Ask = 0,
    Bid = 1,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum OrderType {
    Limit = 0,
    Market = 1,
}
#[doc = r" Generated client implementations."]
pub mod matchengine_client {
    #![allow(unused_variables, dead_code, missing_docs)]
    use tonic::codegen::*;
    pub struct MatchengineClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl MatchengineClient<tonic::transport::Channel> {
        #[doc = r" Attempt to create a new client by connecting to a given endpoint."]
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> MatchengineClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::ResponseBody: Body + HttpBody + Send + 'static,
        T::Error: Into<StdError>,
        <T::ResponseBody as HttpBody>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor(inner: T, interceptor: impl Into<tonic::Interceptor>) -> Self {
            let inner = tonic::client::Grpc::with_interceptor(inner, interceptor);
            Self { inner }
        }
        pub async fn register_user(
            &mut self,
            request: impl tonic::IntoRequest<super::UserInfo>,
        ) -> Result<tonic::Response<super::UserInfo>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/RegisterUser");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn balance_query(
            &mut self,
            request: impl tonic::IntoRequest<super::BalanceQueryRequest>,
        ) -> Result<tonic::Response<super::BalanceQueryResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/BalanceQuery");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn balance_update(
            &mut self,
            request: impl tonic::IntoRequest<super::BalanceUpdateRequest>,
        ) -> Result<tonic::Response<super::BalanceUpdateResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/BalanceUpdate");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn asset_list(
            &mut self,
            request: impl tonic::IntoRequest<super::AssetListRequest>,
        ) -> Result<tonic::Response<super::AssetListResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/AssetList");
            self.inner.unary(request.into_request(), path, codec).await
        }
        #[doc = " rpc AssetSummary(AssetSummaryRequest) returns (AssetSummaryResponse) {}"]
        pub async fn order_put(
            &mut self,
            request: impl tonic::IntoRequest<super::OrderPutRequest>,
        ) -> Result<tonic::Response<super::OrderInfo>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/OrderPut");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn order_query(
            &mut self,
            request: impl tonic::IntoRequest<super::OrderQueryRequest>,
        ) -> Result<tonic::Response<super::OrderQueryResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/OrderQuery");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn order_cancel(
            &mut self,
            request: impl tonic::IntoRequest<super::OrderCancelRequest>,
        ) -> Result<tonic::Response<super::OrderInfo>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/OrderCancel");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn order_cancel_all(
            &mut self,
            request: impl tonic::IntoRequest<super::OrderCancelAllRequest>,
        ) -> Result<tonic::Response<super::OrderCancelAllResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/OrderCancelAll");
            self.inner.unary(request.into_request(), path, codec).await
        }
        #[doc = " rpc OrderBook(OrderBookRequest) returns (OrderBookResponse) {}"]
        pub async fn order_book_depth(
            &mut self,
            request: impl tonic::IntoRequest<super::OrderBookDepthRequest>,
        ) -> Result<tonic::Response<super::OrderBookDepthResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/OrderBookDepth");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn order_detail(
            &mut self,
            request: impl tonic::IntoRequest<super::OrderDetailRequest>,
        ) -> Result<tonic::Response<super::OrderInfo>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/OrderDetail");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn market_list(
            &mut self,
            request: impl tonic::IntoRequest<super::MarketListRequest>,
        ) -> Result<tonic::Response<super::MarketListResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/MarketList");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn reload_markets(
            &mut self,
            request: impl tonic::IntoRequest<super::ReloadMarketsRequest>,
        ) -> Result<tonic::Response<super::SimpleSuccessResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/ReloadMarkets");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn market_summary(
            &mut self,
            request: impl tonic::IntoRequest<super::MarketSummaryRequest>,
        ) -> Result<tonic::Response<super::MarketSummaryResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/MarketSummary");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn transfer(
            &mut self,
            request: impl tonic::IntoRequest<super::TransferRequest>,
        ) -> Result<tonic::Response<super::TransferResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/Transfer");
            self.inner.unary(request.into_request(), path, codec).await
        }
        #[doc = " Used only in development"]
        pub async fn debug_dump(
            &mut self,
            request: impl tonic::IntoRequest<super::DebugDumpRequest>,
        ) -> Result<tonic::Response<super::DebugDumpResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/DebugDump");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn debug_reset(
            &mut self,
            request: impl tonic::IntoRequest<super::DebugResetRequest>,
        ) -> Result<tonic::Response<super::DebugResetResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/DebugReset");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn debug_reload(
            &mut self,
            request: impl tonic::IntoRequest<super::DebugReloadRequest>,
        ) -> Result<tonic::Response<super::DebugReloadResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| tonic::Status::new(tonic::Code::Unknown, format!("Service was not ready: {}", e.into())))?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/matchengine.Matchengine/DebugReload");
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
    impl<T: Clone> Clone for MatchengineClient<T> {
        fn clone(&self) -> Self {
            Self { inner: self.inner.clone() }
        }
    }
    impl<T> std::fmt::Debug for MatchengineClient<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "MatchengineClient {{ ... }}")
        }
    }
}
#[doc = r" Generated server implementations."]
pub mod matchengine_server {
    #![allow(unused_variables, dead_code, missing_docs)]
    use tonic::codegen::*;
    #[doc = "Generated trait containing gRPC methods that should be implemented for use with MatchengineServer."]
    #[async_trait]
    pub trait Matchengine: Send + Sync + 'static {
        async fn register_user(&self, request: tonic::Request<super::UserInfo>) -> Result<tonic::Response<super::UserInfo>, tonic::Status>;
        async fn balance_query(
            &self,
            request: tonic::Request<super::BalanceQueryRequest>,
        ) -> Result<tonic::Response<super::BalanceQueryResponse>, tonic::Status>;
        async fn balance_update(
            &self,
            request: tonic::Request<super::BalanceUpdateRequest>,
        ) -> Result<tonic::Response<super::BalanceUpdateResponse>, tonic::Status>;
        async fn asset_list(
            &self,
            request: tonic::Request<super::AssetListRequest>,
        ) -> Result<tonic::Response<super::AssetListResponse>, tonic::Status>;
        #[doc = " rpc AssetSummary(AssetSummaryRequest) returns (AssetSummaryResponse) {}"]
        async fn order_put(
            &self,
            request: tonic::Request<super::OrderPutRequest>,
        ) -> Result<tonic::Response<super::OrderInfo>, tonic::Status>;
        async fn order_query(
            &self,
            request: tonic::Request<super::OrderQueryRequest>,
        ) -> Result<tonic::Response<super::OrderQueryResponse>, tonic::Status>;
        async fn order_cancel(
            &self,
            request: tonic::Request<super::OrderCancelRequest>,
        ) -> Result<tonic::Response<super::OrderInfo>, tonic::Status>;
        async fn order_cancel_all(
            &self,
            request: tonic::Request<super::OrderCancelAllRequest>,
        ) -> Result<tonic::Response<super::OrderCancelAllResponse>, tonic::Status>;
        #[doc = " rpc OrderBook(OrderBookRequest) returns (OrderBookResponse) {}"]
        async fn order_book_depth(
            &self,
            request: tonic::Request<super::OrderBookDepthRequest>,
        ) -> Result<tonic::Response<super::OrderBookDepthResponse>, tonic::Status>;
        async fn order_detail(
            &self,
            request: tonic::Request<super::OrderDetailRequest>,
        ) -> Result<tonic::Response<super::OrderInfo>, tonic::Status>;
        async fn market_list(
            &self,
            request: tonic::Request<super::MarketListRequest>,
        ) -> Result<tonic::Response<super::MarketListResponse>, tonic::Status>;
        async fn reload_markets(
            &self,
            request: tonic::Request<super::ReloadMarketsRequest>,
        ) -> Result<tonic::Response<super::SimpleSuccessResponse>, tonic::Status>;
        async fn market_summary(
            &self,
            request: tonic::Request<super::MarketSummaryRequest>,
        ) -> Result<tonic::Response<super::MarketSummaryResponse>, tonic::Status>;
        async fn transfer(
            &self,
            request: tonic::Request<super::TransferRequest>,
        ) -> Result<tonic::Response<super::TransferResponse>, tonic::Status>;
        #[doc = " Used only in development"]
        async fn debug_dump(
            &self,
            request: tonic::Request<super::DebugDumpRequest>,
        ) -> Result<tonic::Response<super::DebugDumpResponse>, tonic::Status>;
        async fn debug_reset(
            &self,
            request: tonic::Request<super::DebugResetRequest>,
        ) -> Result<tonic::Response<super::DebugResetResponse>, tonic::Status>;
        async fn debug_reload(
            &self,
            request: tonic::Request<super::DebugReloadRequest>,
        ) -> Result<tonic::Response<super::DebugReloadResponse>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct MatchengineServer<T: Matchengine> {
        inner: _Inner<T>,
    }
    struct _Inner<T>(Arc<T>, Option<tonic::Interceptor>);
    impl<T: Matchengine> MatchengineServer<T> {
        pub fn new(inner: T) -> Self {
            let inner = Arc::new(inner);
            let inner = _Inner(inner, None);
            Self { inner }
        }
        pub fn with_interceptor(inner: T, interceptor: impl Into<tonic::Interceptor>) -> Self {
            let inner = Arc::new(inner);
            let inner = _Inner(inner, Some(interceptor.into()));
            Self { inner }
        }
    }
    impl<T, B> Service<http::Request<B>> for MatchengineServer<T>
    where
        T: Matchengine,
        B: HttpBody + Send + Sync + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = Never;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/matchengine.Matchengine/RegisterUser" => {
                    #[allow(non_camel_case_types)]
                    struct RegisterUserSvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::UserInfo> for RegisterUserSvc<T> {
                        type Response = super::UserInfo;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::UserInfo>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).register_user(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = RegisterUserSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/BalanceQuery" => {
                    #[allow(non_camel_case_types)]
                    struct BalanceQuerySvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::BalanceQueryRequest> for BalanceQuerySvc<T> {
                        type Response = super::BalanceQueryResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::BalanceQueryRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).balance_query(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = BalanceQuerySvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/BalanceUpdate" => {
                    #[allow(non_camel_case_types)]
                    struct BalanceUpdateSvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::BalanceUpdateRequest> for BalanceUpdateSvc<T> {
                        type Response = super::BalanceUpdateResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::BalanceUpdateRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).balance_update(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = BalanceUpdateSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/AssetList" => {
                    #[allow(non_camel_case_types)]
                    struct AssetListSvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::AssetListRequest> for AssetListSvc<T> {
                        type Response = super::AssetListResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::AssetListRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).asset_list(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = AssetListSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/OrderPut" => {
                    #[allow(non_camel_case_types)]
                    struct OrderPutSvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::OrderPutRequest> for OrderPutSvc<T> {
                        type Response = super::OrderInfo;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::OrderPutRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).order_put(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = OrderPutSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/OrderQuery" => {
                    #[allow(non_camel_case_types)]
                    struct OrderQuerySvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::OrderQueryRequest> for OrderQuerySvc<T> {
                        type Response = super::OrderQueryResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::OrderQueryRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).order_query(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = OrderQuerySvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/OrderCancel" => {
                    #[allow(non_camel_case_types)]
                    struct OrderCancelSvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::OrderCancelRequest> for OrderCancelSvc<T> {
                        type Response = super::OrderInfo;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::OrderCancelRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).order_cancel(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = OrderCancelSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/OrderCancelAll" => {
                    #[allow(non_camel_case_types)]
                    struct OrderCancelAllSvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::OrderCancelAllRequest> for OrderCancelAllSvc<T> {
                        type Response = super::OrderCancelAllResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::OrderCancelAllRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).order_cancel_all(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = OrderCancelAllSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/OrderBookDepth" => {
                    #[allow(non_camel_case_types)]
                    struct OrderBookDepthSvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::OrderBookDepthRequest> for OrderBookDepthSvc<T> {
                        type Response = super::OrderBookDepthResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::OrderBookDepthRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).order_book_depth(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = OrderBookDepthSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/OrderDetail" => {
                    #[allow(non_camel_case_types)]
                    struct OrderDetailSvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::OrderDetailRequest> for OrderDetailSvc<T> {
                        type Response = super::OrderInfo;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::OrderDetailRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).order_detail(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = OrderDetailSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/MarketList" => {
                    #[allow(non_camel_case_types)]
                    struct MarketListSvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::MarketListRequest> for MarketListSvc<T> {
                        type Response = super::MarketListResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::MarketListRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).market_list(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = MarketListSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/ReloadMarkets" => {
                    #[allow(non_camel_case_types)]
                    struct ReloadMarketsSvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::ReloadMarketsRequest> for ReloadMarketsSvc<T> {
                        type Response = super::SimpleSuccessResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::ReloadMarketsRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).reload_markets(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = ReloadMarketsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/MarketSummary" => {
                    #[allow(non_camel_case_types)]
                    struct MarketSummarySvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::MarketSummaryRequest> for MarketSummarySvc<T> {
                        type Response = super::MarketSummaryResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::MarketSummaryRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).market_summary(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = MarketSummarySvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/Transfer" => {
                    #[allow(non_camel_case_types)]
                    struct TransferSvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::TransferRequest> for TransferSvc<T> {
                        type Response = super::TransferResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::TransferRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).transfer(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = TransferSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/DebugDump" => {
                    #[allow(non_camel_case_types)]
                    struct DebugDumpSvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::DebugDumpRequest> for DebugDumpSvc<T> {
                        type Response = super::DebugDumpResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::DebugDumpRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).debug_dump(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = DebugDumpSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/DebugReset" => {
                    #[allow(non_camel_case_types)]
                    struct DebugResetSvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::DebugResetRequest> for DebugResetSvc<T> {
                        type Response = super::DebugResetResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::DebugResetRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).debug_reset(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = DebugResetSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/matchengine.Matchengine/DebugReload" => {
                    #[allow(non_camel_case_types)]
                    struct DebugReloadSvc<T: Matchengine>(pub Arc<T>);
                    impl<T: Matchengine> tonic::server::UnaryService<super::DebugReloadRequest> for DebugReloadSvc<T> {
                        type Response = super::DebugReloadResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::DebugReloadRequest>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).debug_reload(request).await };
                            Box::pin(fut)
                        }
                    }
                    let inner = self.inner.clone();
                    let fut = async move {
                        let interceptor = inner.1.clone();
                        let inner = inner.0;
                        let method = DebugReloadSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = if let Some(interceptor) = interceptor {
                            tonic::server::Grpc::with_interceptor(codec, interceptor)
                        } else {
                            tonic::server::Grpc::new(codec)
                        };
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => Box::pin(async move {
                    Ok(http::Response::builder()
                        .status(200)
                        .header("grpc-status", "12")
                        .header("content-type", "application/grpc")
                        .body(tonic::body::BoxBody::empty())
                        .unwrap())
                }),
            }
        }
    }
    impl<T: Matchengine> Clone for MatchengineServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self { inner }
        }
    }
    impl<T: Matchengine> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone(), self.1.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Matchengine> tonic::transport::NamedService for MatchengineServer<T> {
        const NAME: &'static str = "matchengine.Matchengine";
    }
}
