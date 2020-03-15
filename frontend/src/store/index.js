import Vuex from "vuex";
import Vue from "vue";

import call_api from "../api";

Vue.use(Vuex);

const store = new Vuex.Store({
  state: {
    auth_token: localStorage.getItem("auth_token") || null,
    user: {},
  },
  mutations: {
    logged_in(state, { token }) {
      state.auth_token = token;
      localStorage.setItem("auth_token", token);
    },
    logged_out(state) {
      state.auth_token = null;
      localStorage.removeItem("auth_token");
    },
    user_info_fethced(state, user) {
      state.user = user;
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
            commit("logged_in", { token: resp.token });
            store.dispatch("fetch_user_info");
            resolve(resp);
          })
          .catch(err => {
            reject(err);
          });
      });
    },

    fetch_user_info({ commit }) {
      return new Promise((resolve, reject) => {
        call_api({
          path: "user/",
          method: "GET",
        })
          .then(resp => {
            commit("user_info_fethced", resp);
            resolve(resp);
          })
          .catch(err => reject(err));
      });
    },
  },
  getters: {
    is_logged_in: state => !!state.auth_token,
  },
});

export default store;

store.dispatch("fetch_user_info");
