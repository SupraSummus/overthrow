import Vuex from "vuex";
import Vue from "vue";

import call_api from "../api";

Vue.use(Vuex);

const store = new Vuex.Store({
  state: {
    auth: {
      token: localStorage.getItem("auth_token") || null,
      username: localStorage.getItem("auth_username") || null,
    },
  },
  mutations: {
    logged_in(state, { token, username }) {
      state.auth.token = token;
      state.auth.username = username;
      localStorage.setItem("auth_token", token);
      localStorage.setItem("auth_username", username);
    },
    logged_out(state) {
      state.auth.token = null;
      state.auth.username = null;
      localStorage.removeItem("auth_token");
      localStorage.removeItem("auth_username");
    },
  },
  actions: {
    login({ commit }, { username, password }) {
      return new Promise((resolve, reject) => {
        call_api({
          path: "token-auth/",
          method: "POST",
          payload: { username, password },
        })
          .then(resp => {
            commit("logged_in", { token: resp.token, username });
            resolve(resp);
          })
          .catch(err => {
            reject(err);
          });
      });
    },
  },
  getters: {
    is_logged_in: state => !!state.auth.token,
  },
});

export default store;
