import {useState,type ReactNode} from 'react';import {Brand} from './Brand';import {Footer} from './Footer';import {MenuPanel} from './MenuPanel';import {SettingsPanel} from './SettingsPanel';import {navigation} from '../navigation';import {useI18n} from '../i18n';import {useTheme} from '../theme';

const GROUPS=[
  {id:'intelligence',items:['command','intelligence','engines']},
  {id:'analysis',items:['performance','learning','scanner']},
  {id:'operations',items:['alerts','history','health','settings','admin']},
];

const groupCopy:Record<string,Record<string,string>>={
  tr:{intelligence:'İSTİHBARAT',analysis:'ANALİZ',operations:'OPERASYONLAR'},
  en:{intelligence:'INTELLIGENCE',analysis:'ANALYSIS',operations:'OPERATIONS'},
  de:{intelligence:'INTELLIGENZ',analysis:'ANALYSE',operations:'BETRIEB'},
  fr:{intelligence:'RENSEIGNEMENT',analysis:'ANALYSE',operations:'OPÉRATIONS'},
  ar:{intelligence:'الاستخبارات',analysis:'التحليل',operations:'العمليات'},
  ru:{intelligence:'РАЗВЕДКА',analysis:'АНАЛИЗ',operations:'ОПЕРАЦИИ'},
};
const themeCopy:Record<string,Record<string,string>>={
  tr:{light:'Açık tema',dark:'Koyu tema',system:'Sistem teması'},en:{light:'Light theme',dark:'Dark theme',system:'System theme'},
  de:{light:'Helles Design',dark:'Dunkles Design',system:'Systemdesign'},fr:{light:'Thème clair',dark:'Thème sombre',system:'Thème système'},
  ar:{light:'السمة الفاتحة',dark:'السمة الداكنة',system:'سمة النظام'},ru:{light:'Светлая тема',dark:'Тёмная тема',system:'Системная тема'},
};

export function AppShell({active,onNavigate,onSignOut,isAdmin,accountLabel,children}:{active:string;onNavigate:(id:string)=>void;onSignOut:()=>void;isAdmin?:boolean;accountLabel?:string;children:ReactNode}){
  const{t,lang}=useI18n();
  const{preference,effectiveTheme,cycleTheme}=useTheme();
  const[collapsed,setCollapsed]=useState(false);
  const[open,setOpen]=useState<'menu'|'settings'|'profile'|null>(null);
  const toggle=(panel:'menu'|'settings'|'profile')=>setOpen(v=>v===panel?null:panel);
  const go=(id:string)=>{onNavigate(id);setOpen(null)};
  const navLabels:Record<string,string>={command:t.navCommand,intelligence:t.navIntelligence,engines:t.navEngines,performance:t.navPerformance,learning:t.navLearning,scanner:t.navScanner,alerts:t.navAlerts,history:t.navHistory,health:t.navHealth,settings:t.navSettings,admin:t.adminNav};
  const allItems=isAdmin?[...navigation,{id:'admin',icon:'✓'}]:navigation;
  const activeLabel=navLabels[active]??t.navCommand;
  const groupLabels=groupCopy[lang]??groupCopy.en;
  const themeLabels=themeCopy[lang]??themeCopy.en;
  const themeIcon=preference==='system'?'◐':effectiveTheme==='light'?'☀':'☾';

  return <div className={`app-shell workspace-shell ${collapsed?'is-collapsed':''}`}>
    <aside className="sidebar workspace-sidebar">
      <div className="sidebar-brand"><Brand compact={collapsed}/><button className="icon-button" onClick={()=>setCollapsed(v=>!v)} aria-label={t.toggleMenu}>{collapsed?'›':'‹'}</button></div>
      <nav className="workspace-nav">{GROUPS.map(group=>{
        const visible=allItems.filter(item=>group.items.includes(item.id));
        if(!visible.length)return null;
        return <div className="nav-group" key={group.id}>{!collapsed&&<span className="nav-group-title">{groupLabels[group.id]}</span>}{visible.map(item=>{const label=navLabels[item.id];return <button key={item.id} className={active===item.id?'nav-item active':'nav-item'} onClick={()=>go(item.id)} title={label}><span className="nav-icon">{item.icon}</span>{!collapsed&&<span>{label}</span>}</button>})}</div>;
      })}</nav>
      <div className="sidebar-status"><i/>{!collapsed&&<span>{t.intelligenceOnline}</span>}</div>
    </aside>
    <div className="app-column workspace-column">
      <header className="topbar workspace-topbar">
        <div className="workspace-context"><span className="workspace-kicker">BOLDTRACE</span><strong>{activeLabel}</strong><small><span className="pulse"/>{t.liveMarketIntelligence}</small></div>
        <div className="top-actions">
          <button className="theme-cycle" onClick={cycleTheme} aria-label={themeLabels[preference]} title={themeLabels[preference]}><span>{themeIcon}</span><b>{preference.toUpperCase()}</b></button>
          <button className="icon-button" onClick={()=>toggle('menu')} aria-label={t.menuTooltip} title={t.menuTooltip}>☰</button>
          <button className="icon-button" onClick={()=>toggle('settings')} aria-label={t.navSettings} title={t.navSettings}>⚙</button>
          <button className="profile-button" onClick={()=>toggle('profile')}><span>BT</span><div><strong>{accountLabel||t.operator}</strong><small>{t.secureSession}</small></div></button>
        </div>
        {open==='menu'&&<MenuPanel onNavigate={onNavigate} onClose={()=>setOpen(null)}/>}
        {open==='settings'&&<SettingsPanel onOpenSettings={()=>go('settings')} onClose={()=>setOpen(null)}/>}
        {open==='profile'&&<div className="popover profile-menu"><button onClick={()=>go('settings')}>{t.profileAccount}</button><button onClick={()=>go('settings')}>{t.securitySessions}</button><button onClick={()=>go('settings')}>{t.navSettings}</button><hr/><button onClick={onSignOut}>{t.signOut}</button></div>}
      </header>
      <main className="content workspace-content">{children}</main>
      <Footer/>
    </div>
  </div>;
}
