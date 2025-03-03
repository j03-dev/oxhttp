from jwt import ExpiredSignatureError, InvalidTokenError, decode, encode
import bcrypt

SECRET = "8b78e057cf6bc3e646097e5c0277f5ccaa2d8ac3b6d4a4d8c73c7f6af02f0ccd"


def create_jwt(user_id: int) -> str:
    payload = {"user_id": user_id}
    return encode(payload, SECRET, algorithm="HS256")


def decode_jwt(token: str):
    try:
        return decode(token, SECRET, algorithms=["HS256"])
    except (ExpiredSignatureError, InvalidTokenError):
        return None


def hash_password(password: str) -> str:
    return bcrypt.hashpw(password.encode(), bcrypt.gensalt()).decode()


def check_password(hashed_password: str, password: str) -> bool:
    return bcrypt.checkpw(password.encode(), hashed_password.encode())
