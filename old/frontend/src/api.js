import store from "./store";

const api_url = "/api/";

class APIClientError extends Error {
  constructor(response, ...params) {
    super(...params);
    this.response = response;
    this.name = "APIClientError";
  }
}
export { APIClientError };

const call_api = ({ method, path, payload }) => {
  const headers = { "Content-Type": "application/json" };
  if (store.state.auth_token) {
    headers["Authorization"] = "Token " + store.state.auth_token;
  }
  return fetch(api_url + path, {
    method,
    headers,
    body: JSON.stringify(payload),
  }).then(response => {
    // ok but no content
    if (response.status == 204) return null;

    // ok with content
    if (response.ok) return response.json();

    // client's fault
    if (response.status >= 400 && response.status < 500) {
      return response.json().then(message => {
        const e = new APIClientError(message);
        throw e;
      });
    }

    console.error("unexpected response", response);
    throw response;
  });
};

export default call_api;
