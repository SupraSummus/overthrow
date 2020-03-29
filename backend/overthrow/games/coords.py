def distance(a, b):
    """ distance in cube coordinates. """
    return max(map(abs, subtract(a, b)))


def subtract(a, b):
    return tuple(map(lambda v: v[0] - v[1], zip(a, b)))


def sum(a, b):
    return tuple(map(lambda v: v[0] + v[1], zip(a, b)))


def as_dict(v):
    return dict(zip(('x', 'y', 'z'), v))


def _sign(x):
    if x > 0:
        return 1
    if x < 0:
        return -1
    return 0


def step_towards(src, dst):
    d = list(subtract(dst, src))
    a = list(map(abs, d))
    i = a.index(min(a))
    d[i] = 0
    d = tuple(map(_sign, d))
    assert __builtins__['sum'](d) == 0, d
    return d


def next_on_path(src, dst):
    return sum(src, step_towards(src, dst))
