import {useState,type ReactNode} from 'react';import {Brand} from './Brand';import {Footer} from './Footer';import {MenuPanel} from './MenuPanel';import {SettingsPanel} from './SettingsPanel';import {navigation} from '../navigation';import {useI18n} from '../i18n';
export function AppShell({active,onNavigate,onSignOut,isAdmin,accountLabel,children}:{active:string;onNavigate:(id:string)=>void;onSignOut:()=>void;isAdmin?:boolean;accountLabel?:string;children:ReactNode}){
  const{t}=useI18n();
  const[collapsed,setCollapsed]=useState(false);
  const[open,setOpen]=useState<'menu'|'settings'|'profile'|null>(null);
  const toggle=(panel:'menu'|'settings'|'profile')=>setOpen(v=>v===panel?null:panel);
  const go=(id:string)=>{onNavigate(id);setOpen(null)};
  const navLabels:Record<string,string>={command:t.navCommand,intelligence:t.navIntelligence,engines:t.navEngines,performance:t.navPerformance,learning:t.navLearning,scanner:t.navScanner,alerts:t.navAlerts,history:t.navHistory,health:t.navHealth,settings:t.navSettings};
  const items=isAdmin?[...navigation,{id:'admin',icon:'✓'}]:navigation;
  return <div className={`app-shell ${collapsed?'is-collapsed':''}`}>
    <aside className="sidebar">
      <div className="sidebar-brand"><Brand compact={collapsed}/><button className="icon-button" onClick={()=>setCollapsed(v=>!v)} aria-label={t.toggleMenu}>{collapsed?'›':'‹'}</button></div>
      <nav>{items.map(item=>{const label=item.id==='admin'?t.adminNav:navLabels[item.id];return <button key={item.id} className={active===item.id?'nav-item active':'nav-item'} onClick={()=>go(item.id)} title={label}><span className="nav-icon">{item.icon}</span>{!collapsed&&<span>{label}</span>}</button>})}</nav>
      <div className="sidebar-status"><i/>{!collapsed&&<span>{t.intelligenceOnline}</span>}</div>
    </aside>
    <div className="app-column">
      <header className="topbar">
        <div className="market-state"><span className="pulse"/>{t.liveMarketIntelligence}</div>
        <div className="top-actions">
          <button className="icon-button" onClick={()=>toggle('menu')} aria-label={t.menuTooltip} title={t.menuTooltip}>☰</button>
          <button className="icon-button" onClick={()=>toggle('settings')} aria-label={t.navSettings} title={t.navSettings}>⚙</button>
          <button className="profile-button" onClick={()=>toggle('profile')}><span>BT</span><div><strong>{accountLabel||t.operator}</strong><small>{t.secureSession}</small></div></button>
        </div>
        {open==='menu'&&<MenuPanel onNavigate={onNavigate} onClose={()=>setOpen(null)}/>}
        {open==='settings'&&<SettingsPanel onOpenSettings={()=>go('settings')} onClose={()=>setOpen(null)}/>}
        {open==='profile'&&<div className="popover profile-menu"><button onClick={()=>go('settings')}>{t.profileAccount}</button><button onClick={()=>go('settings')}>{t.securitySessions}</button><button onClick={()=>go('settings')}>{t.navSettings}</button><hr/><button onClick={onSignOut}>{t.signOut}</button></div>}
      </header>
      <main className="content">{children}</main>
      <Footer/>
    </div>
  </div>;
}
