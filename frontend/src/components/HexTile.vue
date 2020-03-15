<template>
  <div class="tile" :style="position_style">
    <div class="tile_content">
      <div class="tile_army">{{ tile.army }}</div>
      <div v-if="owned">owned</div>
      <div class="tile_coords">{{ tile.x }} / {{ tile.y }} / {{ tile.z }}</div>
    </div>
    <div
      class="tile_border"
      v-for="border_id in visible_borders"
      v-bind:key="border_id"
      v-bind:style="{
        color: 'blue',
        transform: `rotate(${get_border_rotation(
          border_id,
        )}) translateY(${-tile_size / 2 + border_spacing}px) `,
      }"
    ></div>
  </div>
</template>

<script>
const tile_size = 100; // px
const tile_height = (tile_size / Math.sqrt(3)) * 2; //px
const border_spacing = 5; //px

export default {
  props: ["tile", "players"],
  data: function() {
    return { tile_size, border_spacing };
  },
  computed: {
    position_style: function() {
      return {
        top: this.tile.y * tile_height * 0.75 - tile_height / 2 + "px",
        left: (this.tile.x + this.tile.y / 2 - 0.5) * tile_size + "px",
      };
    },
    visible_borders: function() {
      let visible_borders = [];
      for (let border_id in this.tile.borders) {
        if (this.tile.borders[border_id]) visible_borders.push(border_id);
      }
      return visible_borders;
    },
    owned: function() {
      if (this.tile.owner === null) {
        return false;
      } else {
        return this.players[this.tile.owner].user == this.$store.state.user.id;
      }
    },
  },
  methods: {
    get_border_rotation: function(border_id) {
      return {
        "-1_0_1": "270deg",
        "-1_1_0": "210deg",
        "0_-1_1": "330deg",
        "0_1_-1": "150deg",
        "1_-1_0": "30deg",
        "1_0_-1": "90deg",
      }[border_id];
    },
  },
};
</script>

<style scoped lang="scss">
$tile-size: 100px;
$border-spacing: 5px;

$sqrt3: 1.7320508075688772;
$tile-side: $tile-size / $sqrt3;
$tile-height: $tile-size * 2 / $sqrt3;
$border-width: $tile-side - $border-spacing * 2;

.tile {
  position: absolute;
  width: $tile-size;
  height: $tile-height;
}

.tile_content {
  margin: 0;
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  text-align: center;
}

.tile_army {
  font-size: 40px;
}

.tile_coords {
  font-size: 5px;
}

.tile_border {
  border-width: 0;
  border-bottom: 2px dashed;
  width: $border-width;
  position: absolute;
  top: $tile-height / 2;
  left: ($tile-size - $border-width) / 2;
}
</style>
