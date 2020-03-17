<template>
  <v-app>
    <v-app-bar app dense>
      <!-- <v-app-bar-nav-icon/> -->
      <v-toolbar-title>overthrow</v-toolbar-title>

      <login v-if="!$store.getters.is_logged_in" />
      <v-spacer />

      <v-menu left bottom>
        <template v-slot:activator="{ on }">
          <v-btn icon v-on="on">
            <v-icon>mdi-dots-vertical</v-icon>
          </v-btn>
        </template>

        <v-list>
          <v-list-item
            v-if="$store.getters.is_logged_in"
            :to="{ name: 'game', params: { id: 'default' } }"
          >
            <v-list-item-action>
              a map
            </v-list-item-action>
          </v-list-item>
          <v-list-item v-if="$store.getters.is_logged_in">
            <v-list-item-content>
              <v-list-title>
                Logged in as {{ $store.state.user.username }}
              </v-list-title>
            </v-list-item-content>
          </v-list-item>
          <v-list-item
            v-if="$store.getters.is_logged_in"
            @click="$store.commit('logged_out')"
          >
            <v-list-item-action>
              logout
            </v-list-item-action>
          </v-list-item>
        </v-list>
      </v-menu>
    </v-app-bar>

    <v-content>
      <router-view />
    </v-content>
  </v-app>
</template>

<script>
import Login from "./components/Login.vue";

export default {
  components: {
    Login,
  },
};
</script>
