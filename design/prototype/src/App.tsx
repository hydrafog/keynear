import { AppProvider } from './state/AppProvider';
import { AppShell } from './features/system/AppShell';

export default function App() {
  return (
    <AppProvider>
      <AppShell />
    </AppProvider>
  );
}
