from oxhttp import Status
from utils import decode_jwt


def logger(request, next, **kwargs):
    method = request.method
    host = request.headers.get("host")
    uri = request.uri
    print(f"method:{method} host:{host} uri:{uri}")
    return next(**kwargs)


def jwt_middleware(request, next, **kwargs):
    token = request.headers.get("authorization", "").replace("Bearer ", "")

    if token:
        if payload := decode_jwt(token):
            kwargs["user_id"] = payload["user_id"]
            return next(**kwargs)
    return Status.UNAUTHORIZED
