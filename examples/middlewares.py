from oxapy import Status
from utils import decode_jwt


def jwt_middleware(request, next, **kwargs):
    token = request.headers.get("authorization", "").replace("Bearer ", "")

    if token:
        if payload := decode_jwt(token):
            kwargs["user_id"] = payload["user_id"]
            return next(**kwargs)
    return Status.UNAUTHORIZED


def logger(request, next, **kwargs):
    print(request.method, request.uri)
    return next(**kwargs)
