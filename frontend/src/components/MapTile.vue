<template>
  <div class="tile" v-bind:style="position_style">
    <div
      class="tile-content"
      v-on:click="$emit('select')"
      v-on:mouseover="$emit('hover')"
      v-bind:class="{ selected: selected }"
    >
      <div class="tile-army">{{ army_without_outgoing }}</div>
      <div class="tile-coords" v-if="owned">owned</div>
      <div class="tile-coords">{{ x }} / {{ y }} / {{ z }}</div>
    </div>
    <spinner v-if="processing > 0" class="tile-spinner" />

    <!-- borders -->
    <div
      class="tile-border"
      v-for="border_id in borders"
      v-bind:key="border_id"
      v-bind:style="{
        color: 'blue',
        transform: `rotate(${get_border_rotation(
          border_id,
        )}) translateY(${-tile_size / 2 + border_spacing}px)`,
      }"
    />
  </div>
</template>

<script>
import Spinner from "vue-simple-spinner";

import { delta_rotations, tile_size, tile_height } from "@/constants";

const border_spacing = 5; //px

export default {
  props: [
    "x",
    "y",
    "z",
    "army",
    "selected",
    "borders",
    "owned",
    "outgoing_movements",
    "processing",
  ],
  components: { Spinner },
  data: function() {
    return { tile_size, border_spacing };
  },
  computed: {
    position_style: function() {
      return {
        top: this.y * tile_height * 0.75 - tile_height / 2 + "px",
        left: (this.x + this.y / 2 - 0.5) * tile_size + "px",
      };
    },
    army_without_outgoing: function() {
      let army = this.army;
      for (let movement_id in this.outgoing_movements)
        army -= this.outgoing_movements[movement_id].amount;
      return army >= 0 ? army : 0;
    },
  },
  methods: {
    get_border_rotation: function(border_id) {
      return delta_rotations[border_id] + "deg";
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

.tile-content,
.tile-spinner {
  // center in parent container
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
}

.tile-content {
  // center content
  text-align: center;

  padding: 0.5rem;
  border-radius: 10rem;
  transition: background-color 0.5s ease;

  &.selected {
    background-color: lightgray;
  }

  cursor: pointer;
}

.tile-army {
  font-weight: bold;
  font-size: 30px;
}

.tile-coords {
  font-size: 5px;
}

.tile-border {
  border-width: 0;
  border-bottom: 2px dashed;
  width: $border-width;
  position: absolute;
  top: $tile-height / 2;
  left: ($tile-size - $border-width) / 2;
}
</style>
