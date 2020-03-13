import Vuex from "vuex";
import Vue from "vue";

Vue.use(Vuex);

const call_api = ({ method, path, payload }) => {
  const headers = { "Content-Type": "application/json" };
  if (store.state.auth.token) {
    headers["Authorization"] = "Token " + store.state.auth.token;
  }
  return fetch("http://localhost:8000/api/" + path, {
    method,
    headers,
    body: JSON.stringify(payload),
  }).then(response => {
    if (response.ok) {
      return response.json();
    } else if (response.status >= 400 && response.status < 500) {
      throw response.json();
    } else {
      console.error(response);
      throw "unexpected response";
    }
  });
};

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
