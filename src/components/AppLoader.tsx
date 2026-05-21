export function AppLoader({ systemPrefersDark }: { systemPrefersDark: boolean }) {
  return (
    <div className={`app-loader-react ${systemPrefersDark ? "theme-dark" : "theme-light"}`}>
      <div className="app-loader-spinner" />
      <span className="app-loader-label">ledgera</span>
    </div>
  );
}
