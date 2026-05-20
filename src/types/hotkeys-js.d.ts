declare module "hotkeys-js" {
  type KeyHandler = (event: KeyboardEvent, handler: { key: string }) => void;

  interface Hotkeys {
    (key: string, method: KeyHandler): void;
    (key: string, scope: string, method: KeyHandler): void;
    setScope(scope: string): void;
    getScope(): string;
    deleteScope(scope: string): void;
    unbind(key?: string, scope?: string, method?: KeyHandler): void;
    filter: (event: KeyboardEvent) => boolean;
  }

  const hotkeys: Hotkeys;
  export default hotkeys;
}
