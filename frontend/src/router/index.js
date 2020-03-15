import Vue from "vue";
import VueRouter from "vue-router";

import Game from "@/components/Game.vue";

Vue.use(VueRouter);

const routes = [
  { path: "/game/:id", component: Game, name: "game", props: true },
];

const router = new VueRouter({
  routes,
});

export default router;
