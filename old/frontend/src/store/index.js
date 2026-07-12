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
      store.dispatch("fetch_user_info"); // welp, this probably shuda be in actions
    },
    user_info_fetched(state, user) {
      state.user = user;
    },
  },

  actions: {
    log_in({ commit }, { username, password }) {
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

    log_out({ commit }) {
      commit("logged_out");
    },

    fetch_user_info({ commit }) {
      return new Promise((resolve, reject) => {
        call_api({
          path: "user/",
          method: "GET",
        })
          .then(resp => {
            commit("user_info_fetched", resp);
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
