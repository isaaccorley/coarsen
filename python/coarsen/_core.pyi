def simplify_wkb(
    inputs: list[bytes],
    tolerance: float,
    simplify_boundary: bool,
    threads: int | None = None,
) -> list[bytes]: ...
def topology_wkb(
    inputs: list[bytes],
    tolerances: list[float],
    rings: list[list[list[int]]],
    threads: int | None = None,
) -> list[tuple[bytes, list[list[int]]]]: ...
