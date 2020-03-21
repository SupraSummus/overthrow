import Vue from "vue";
import VueRouter from "vue-router";

import Game from "@/components/Game.vue";
import Login from "@/components/Login.vue";
import SignUp from "@/components/sign_up.vue";
import store from "@/store";

Vue.use(VueRouter);

function login_required(to, from, next) {
  if (!store.getters.is_logged_in) {
    next({ name: "login" });
  } else {
    next();
  }
}

const routes = [
  {
    path: "/game/:id",
    component: Game,
    name: "game",
    props: true,
    beforeEnter: login_required,
  },
  {
    path: "/login",
    component: Login,
    name: "login",
  },
  {
    path: "/signup",
    component: SignUp,
    name: "sign_up",
  },
];

const router = new VueRouter({
  routes,
});

export default router;
