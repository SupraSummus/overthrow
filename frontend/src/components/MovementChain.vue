<template>
  <div>
    <movement
      v-for="(step, id) in movement_steps"
      v-bind:key="id"
      v-bind="step"
      v-on:click.native="step_clicked(step)"
    />
  </div>
</template>

<script>
import Movement from "./Movement.vue";
import { coord_delta_one, coord_sum, coord_equal, coord_string } from "@/coord";

export default {
  props: ["source", "target", "amount", "processing"],
  components: { Movement },
  computed: {
    movement_steps: function() {
      let tile = this.source;
      const steps = {};
      while (!coord_equal(tile, this.target)) {
        const delta = coord_delta_one(tile, this.target);
        const first = coord_equal(tile, this.source);
        steps[coord_string(tile)] = {
          source: tile,
          delta,
          amount: first ? this.amount : null,
          processing: first ? this.processing : null,
        };
        tile = coord_sum(tile, delta);
      }
      return steps;
    },
  },
  methods: {
    step_clicked: function(step) {
      if (coord_equal(step.source, this.source)) {
        this.$emit("delete");
      }
    },
  },
};
</script>
