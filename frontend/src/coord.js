export const coord_neighbour_deltas = [
  { x: -1, y: 0, z: 1 },
  { x: -1, y: 1, z: 0 },
  { x: 0, y: -1, z: 1 },
  { x: 0, y: 1, z: -1 },
  { x: 1, y: -1, z: 0 },
  { x: 1, y: 0, z: -1 },
];

export function coord_sum(a, b) {
  return { x: a.x + b.x, y: a.y + b.y, z: a.z + b.z };
}

export function coord_negative(a) {
  return { x: -a.x, y: -a.y, z: -a.z };
}

export function coord_string(coords) {
  return `${coords.x}_${coords.y}_${coords.z}`;
}

export function coord_subtract(a, b) {
  return { x: a.x - b.x, y: a.y - b.y, z: a.z - b.z };
}

export function coord_map(f, { x, y, z }) {
  return { x: f(x), y: f(y), z: f(z) };
}

export function coord_check(v) {
  console.assert(v.x + v.y + v.z == 0, v);
  return v;
}

export function coord_delta_one(source, target) {
  const delta = coord_subtract(target, source);
  const delta_one = coord_map(Math.sign, delta);
  const delta_abs = coord_map(Math.abs, delta);
  if (delta_abs.x <= delta_abs.y && delta_abs.x <= delta_abs.z) delta_one.x = 0;
  else if (delta_abs.y <= delta_abs.x && delta_abs.y <= delta_abs.z)
    delta_one.y = 0;
  else if (delta_abs.z <= delta_abs.x && delta_abs.z <= delta_abs.y)
    delta_one.z = 0;
  return coord_check(delta_one);
}

export function coord_equal(a, b) {
  return a.x == b.x && a.y == b.y && a.z == b.z;
}
