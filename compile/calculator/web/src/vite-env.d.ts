/// <reference types="vite/client" />

declare module "*.calc?raw" {
  const source: string;
  export default source;
}
