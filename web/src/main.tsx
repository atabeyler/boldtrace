import React,{useEffect,useState}from'react';import{createRoot}from'react-dom/client';import{I18nProvider}from'./i18n';import{AuthPage}from'./pages/AuthPage';import{CommandCenter}from'./pages/CommandCenter';import{IntelligenceTerminal}from'./pages/IntelligenceTerminal';import{EngineMatrix}from'./pages/EngineMatrix';import{SettingsPage}from'./pages/SettingsPage';import{PerformanceCenter}from'./pages/PerformanceCenter';import{LearningCenter}from'./pages/LearningCenter';import{MarketScanner,AlertsPage,HistoryPage,SystemHealth}from'./pages/OperationsPages';import{AdminPage}from'./pages/AdminPage';import{AppShell}from'./components/AppShell';import{api}from'./api/client';import type{Account}from'./api/contracts';import'./styles.css';

function App(){
  const[account,setAccount]=useState<Account|null|undefined>(undefined);
  const[page,setPage]=useState('command');

  useEffect(()=>{api.me().then(setAccount).catch(()=>setAccount(null))},[]);

  if(account===undefined)return <div className="auth-boot"/>;
  if(account===null)return <AuthPage onAuthenticated={setAccount}/>;

  const views:Record<string,React.ReactNode>={command:<CommandCenter/>,intelligence:<IntelligenceTerminal/>,engines:<EngineMatrix/>,performance:<PerformanceCenter/>,learning:<LearningCenter/>,scanner:<MarketScanner/>,alerts:<AlertsPage/>,history:<HistoryPage/>,health:<SystemHealth/>,settings:<SettingsPage/>,admin:<AdminPage/>};
  const signOut=()=>{api.logout().catch(()=>{}).finally(()=>setAccount(null))};
  return <AppShell active={page} onNavigate={setPage} onSignOut={signOut} isAdmin={account.isAdmin}>{views[page]??<CommandCenter/>}</AppShell>;
}

createRoot(document.getElementById('root')!).render(<React.StrictMode><I18nProvider><App/></I18nProvider></React.StrictMode>);
