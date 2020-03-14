<template>
  <div>
    <nav>
      <router-link :to="{ name: 'map' }">a map</router-link>
      <login v-if="!$store.getters.is_logged_in" />
      <logout v-if="$store.getters.is_logged_in" />
    </nav>
    <router-view></router-view>
  </div>
</template>

<script>
import Login from "./components/Login.vue";
import Logout from "./components/Logout.vue";

export default {
  components: {
    Login,
    Logout,
  },
};
</script>
