"""Deterministic input hashing (SHA-256, uppercase, chunked reads)."""
import hashlib

CHUNK_SIZE = 1 << 20


def sha256_file(path):
    """Return the uppercase SHA-256 hex digest of a file's contents."""
    digest = hashlib.sha256()
    with open(str(path), 'rb') as stream:
        for chunk in iter(lambda: stream.read(CHUNK_SIZE), b''):
            digest.update(chunk)
    return digest.hexdigest().upper()
