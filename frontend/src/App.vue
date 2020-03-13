<template>
  <div>
    <login v-if="!$store.getters.is_logged_in" />
    <logout v-if="$store.getters.is_logged_in" />
    <hex-grid v-if="$store.getters.is_logged_in" />
  </div>
</template>

<script>
import HexGrid from "./components/HexGrid.vue";
import Login from "./components/Login.vue";
import Logout from "./components/Logout.vue";

export default {
  components: {
    HexGrid,
    Login,
    Logout,
  },
};
</script>
