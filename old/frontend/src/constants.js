export const tile_size = 100; // px
export const tile_height = (tile_size / Math.sqrt(3)) * 2; //px
export const tile_width = tile_size;

// delta coord string -> rotation in degrees
export const delta_rotations = {
  "-1_0_1": 270,
  "-1_1_0": 210,
  "0_-1_1": 330,
  "0_1_-1": 150,
  "1_-1_0": 30,
  "1_0_-1": 90,
};
