/// <reference types="vite/client" />

/** `import x from "./algo.pdf?url"` — Vite lo emite como asset y devuelve su URL. */
declare module "*.pdf?url" {
  const url: string;
  export default url;
}
