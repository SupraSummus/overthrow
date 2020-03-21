<template>
  <b-navbar>
    <template slot="brand">
      <b-navbar-item tag="router-link" :to="{ path: '/' }">
        Overthrow
      </b-navbar-item>
    </template>

    <template slot="start">
      <b-navbar-item
        tag="router-link"
        :to="{ name: 'game', params: { id: 'default' } }"
      >
        Default game
      </b-navbar-item>
    </template>

    <template slot="end">
      <b-navbar-item tag="div" v-if="!$store.getters.is_logged_in">
        <div class="buttons">
          <a class="button is-primary">
            <strong>Sign up</strong>
          </a>
          <a class="button is-light">
            Log in
          </a>
        </div>
      </b-navbar-item>

      <b-navbar-item tag="div" v-if="$store.getters.is_logged_in">
        <div class="buttons">
          <span class="buttons-inline" v-if="$store.state.user.username">
            Logged in as {{ $store.state.user.username }}
          </span>
          <a class="button is-light" v-on:click="$store.dispatch('log_out')">
            Log out
          </a>
        </div>
      </b-navbar-item>
    </template>
  </b-navbar>
</template>

<script>
export default {};
</script>

<style scoped>
.buttons-inline {
  margin-right: 0.75rem;
  margin-bottom: 0.5rem;
}
</style>
