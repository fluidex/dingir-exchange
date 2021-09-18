import matchengine_pb2_grpc
import matchengine_pb2
import ordersigner_pb2_grpc
import ordersigner_pb2
import grpc

uid = 11


signer_channel = grpc.insecure_channel('localhost:50061')
signer_stub = ordersigner_pb2_grpc.OrderSignerStub(signer_channel)


matchengine_channel = grpc.insecure_channel('localhost:50051')
matchengine_stub = matchengine_pb2_grpc.MatchengineStub(matchengine_channel)


def put_order():
    uid = 3
    order = ordersigner_pb2.SignOrderRequest(
        user_id=uid,
        market='ETH_USDT',
        order_side=ordersigner_pb2.OrderSide.BID,
        order_type=ordersigner_pb2.OrderType.LIMIT,
        amount="1",
        price="1000",
    )
    sig = signer_stub.SignOrder(order).signature
    order_final = matchengine_pb2.OrderPutRequest(

        user_id=uid,
        market='ETH_USDT',
        order_side=matchengine_pb2.OrderSide.BID,
        order_type=matchengine_pb2.OrderType.LIMIT,
        amount="1",
        price="1000",
        signature=sig
    )
    res = matchengine_stub.OrderPut(order_final)
    print(res)


put_order()
