<template>
  <div>
    <login v-if="!$store.getters.is_logged_in" />

    <nav class="navbar" v-if="$store.getters.is_logged_in">
      <div class="navbar-item">
        <router-link :to="{ name: 'game', params: { id: 'default' } }"
          >a map</router-link
        >
      </div>
      <div class="navbar-item">
        <logout />
      </div>
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

<style scoped lang="scss">
.navbar {
  background-color: white;
}
.navbar-item {
  display: inline-block;
  margin: 0 4rem;
}
</style>
