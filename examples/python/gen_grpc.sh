python3 -m grpc_tools.protoc -I../js  --python_out=. --grpc_python_out=. ordersigner.proto
python3 -m grpc_tools.protoc -I../../orchestra/proto/exchange -I../../orchestra/proto/third_party/googleapis/ --python_out=. --grpc_python_out=. matchengine.proto
