<template>
  <on-map v-bind="$props">
    <div class="dialog box">
      <slot />
    </div>
  </on-map>
</template>

<script>
import OnMap from "./on_map.vue";

export default {
  props: ["x", "y", "z"],
  components: { OnMap },
};
</script>

<style scoped>
.dialog {
  position: absolute;
  transform: translate(-50%, -50%);

  width: 300px;
}
</style>
