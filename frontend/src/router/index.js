import Vue from "vue";
import VueRouter from "vue-router";

import HexGrid from "@/components/HexGrid.vue";

Vue.use(VueRouter);

const routes = [{ path: "/map", component: HexGrid, name: "map" }];

const router = new VueRouter({
  routes,
});

export default router;
