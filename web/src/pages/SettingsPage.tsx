import {useState} from 'react';import {settingsSections} from '../navigation';import {LanguageSelector} from '../components/LanguageSelector';import {useApi} from '../api/useApi';import {api,ApiError} from '../api/client';import {useI18n} from '../i18n';import type {Copy} from '../i18n';import type {Account} from '../api/contracts';import {sortedCountries} from '../countries';

export function SettingsPage({account,onAccountChange}:{account:Account;onAccountChange:(account:Account)=>void}){
  const{t,lang}=useI18n();
  const[section,setSection]=useState('profile');
  const sectionTitle:Record<string,string>={profile:t.settingsProfile,appearance:t.settingsAppearance,language:t.settingsLanguage,markets:t.settingsMarkets,intelligence:t.settingsIntelligenceSection,notifications:t.settingsNotifications,privacy:t.settingsPrivacy,system:t.settingsSystem};
  const{data:health}=useApi(()=>api.health(),[],20000);
  const notPersisted=section!=='profile'&&section!=='language'&&section!=='system';
  return <div className="page"><div className="page-head"><div><span className="eyebrow">{t.settingsEyebrow}</span><h1>{t.settingsTitle}</h1><p>{t.settingsSub}</p></div></div><div className="settings-layout"><aside className="settings-nav">{settingsSections.map(s=><button className={section===s.id?'active':''} onClick={()=>setSection(s.id)} key={s.id}>{sectionTitle[s.id]}</button>)}</aside><section className="panel settings-panel"><span className="eyebrow">{t.settingsConfiguration}</span><h2>{sectionTitle[section]}</h2>{notPersisted&&<p className="terminal-copy">{t.settingsNotPersisted}</p>}{section==='profile'&&<ProfileSection account={account} onAccountChange={onAccountChange} t={t} lang={lang}/>}{section==='language'&&<><label>{t.settingsInterfaceLanguage}<LanguageSelector/></label></>}{section==='intelligence'&&<><label>{t.settingsDefaultHorizon}<select><option>{t.settingsHorizon60}</option><option>{t.settingsHorizon15}</option><option>{t.settingsHorizon240}</option></select></label><label>{t.settingsMinConfidence}<input type="range" min="50" max="95" defaultValue="70"/></label><label>{t.settingsMaxRisk}<input type="range" min="10" max="90" defaultValue="55"/></label><Toggle title={t.settingsAdaptiveLearning} text={t.settingsAdaptiveLearningHint} on/><Toggle title={t.settingsEngineDetail} text={t.settingsEngineDetailHint} on/></>}{section==='notifications'&&<><Toggle title={t.settingsInAppAlerts} text={t.settingsInAppAlertsHint} on/><Toggle title={t.settingsTelegramAlerts} text={t.settingsTelegramAlertsHint} on/><Toggle title={t.settingsEmailAlerts} text={t.settingsEmailAlertsHint}/><label>{t.settingsMinSeverity}<select><option>WATCH</option><option>HIGH</option><option>CRITICAL</option></select></label></>}{section==='markets'&&<><label>{t.settingsDefaultMarket}<select><option>BTCUSDT</option><option>ETHUSDT</option><option>SOLUSDT</option></select></label><label>{t.settingsQuoteCurrency}<select><option>USDT</option><option>USDC</option></select></label><Toggle title={t.settingsAutoUniverse} text={t.settingsAutoUniverseHint} on/></>}{section==='appearance'&&<><label>{t.settingsTheme}<select><option>{t.settingsThemeMidnight}</option><option>{t.settingsThemeSystem}</option></select></label><label>{t.settingsDensity}<select><option>{t.settingsDensityComfortable}</option><option>{t.settingsDensityCompact}</option></select></label><Toggle title={t.settingsReducedMotion} text={t.settingsReducedMotionHint}/></>}{section==='privacy'&&<><div className="setting-line"><div><b>{t.settingsLoginActivity}</b><small>{t.settingsLoginActivityHint}</small></div><button disabled title={t.settingsComingSoon}>{t.settingsOpen}</button></div><div className="setting-line danger"><div><b>{t.settingsDeleteAccount}</b><small>{t.settingsDeleteAccountHint}</small></div><button disabled title={t.settingsComingSoon}>{t.settingsDelete}</button></div></>}{section==='system'&&(health?health.map(s=><Status key={s.name} n={s.name} status={s.status} t={t}/>):<p>{t.settingsSystemLoading}</p>)}</section></div></div>
}

function ProfileSection({account,onAccountChange,t,lang}:{account:Account;onAccountChange:(account:Account)=>void;t:Copy;lang:string}){
  const countries=sortedCountries(lang);
  const[busy,setBusy]=useState(false);
  const[message,setMessage]=useState<{kind:'ok'|'err';text:string}|null>(null);
  const[pwBusy,setPwBusy]=useState(false);
  const[pwMessage,setPwMessage]=useState<{kind:'ok'|'err';text:string}|null>(null);

  const saveProfile=async(e:React.FormEvent<HTMLFormElement>)=>{
    e.preventDefault();
    setMessage(null);
    setBusy(true);
    const form=new FormData(e.currentTarget);
    try{
      const updated=await api.updateProfile({
        firstName:String(form.get('firstName')||''),
        lastName:String(form.get('lastName')||''),
        userCode:String(form.get('userCode')||''),
        country:String(form.get('country')||''),
        nationalId:String(form.get('nationalId')||''),
      });
      onAccountChange(updated);
      setMessage({kind:'ok',text:t.settingsSaved});
    }catch(err){
      const code=err instanceof ApiError?err.code:'';
      setMessage({kind:'err',text:code==='user_code_taken'?t.errorUserCodeTaken:code==='invalid_user_code'?t.errorInvalidUserCode:t.errorGeneric});
    }finally{
      setBusy(false);
    }
  };

  const savePassword=async(e:React.FormEvent<HTMLFormElement>)=>{
    e.preventDefault();
    setPwMessage(null);
    setPwBusy(true);
    const formEl=e.currentTarget;
    const form=new FormData(formEl);
    try{
      await api.changePassword({
        currentPassword:String(form.get('currentPassword')||''),
        newPassword:String(form.get('newPassword')||''),
      });
      setPwMessage({kind:'ok',text:t.settingsPasswordChanged});
      formEl.reset();
    }catch(err){
      const code=err instanceof ApiError?err.code:'';
      setPwMessage({kind:'err',text:code==='invalid_credentials'?t.settingsCurrentPasswordWrong:t.errorGeneric});
    }finally{
      setPwBusy(false);
    }
  };

  return <>
    <form onSubmit={saveProfile}>
      <label>{t.firstName}<input required name="firstName" defaultValue={account.firstName}/></label>
      <label>{t.lastName}<input required name="lastName" defaultValue={account.lastName}/></label>
      <label>{t.email}<input readOnly type="email" value={account.email}/></label>
      <label>{t.userCode}<input required name="userCode" minLength={4} maxLength={20} pattern="[A-Za-z0-9]{4,20}" defaultValue={account.userCode} title={t.userCodeHint}/></label>
      <label>{t.country}<select required name="country" defaultValue={account.country}>{countries.map(c=><option key={c.code} value={c.code}>{c.name}</option>)}</select></label>
      <label>{t.nationalId}<input required name="nationalId" defaultValue={account.nationalId} title={t.nationalIdHint}/></label>
      {message&&<p className={message.kind==='ok'?'auth-pending':'auth-error'} role="status">{message.text}</p>}
      <button className="primary-small" disabled={busy}>{busy?t.authorizing:t.settingsSave}</button>
    </form>
    <div className="setting-line"><div><b>{t.settingsTwoFactor}</b><small>{t.settingsTwoFactorHint}</small></div><button disabled title={t.settingsComingSoon}>{t.settingsConfigure}</button></div>
    <div className="setting-line"><div><b>{t.settingsActiveSessions}</b><small>{t.settingsActiveSessionsHint}</small></div><button disabled title={t.settingsComingSoon}>{t.settingsReview}</button></div>
    <div className="setting-line-header"><b>{t.settingsChangePassword}</b></div>
    <form onSubmit={savePassword}>
      <label>{t.settingsCurrentPassword}<input required type="password" name="currentPassword" autoComplete="current-password"/></label>
      <label>{t.settingsNewPassword}<input required type="password" name="newPassword" minLength={8} autoComplete="new-password"/></label>
      {pwMessage&&<p className={pwMessage.kind==='ok'?'auth-pending':'auth-error'} role="status">{pwMessage.text}</p>}
      <button className="primary-small" disabled={pwBusy}>{pwBusy?t.authorizing:t.settingsChangePassword}</button>
    </form>
  </>;
}

function Toggle({title,text,on=false}:{title:string;text:string;on?:boolean}){return <div className="setting-line"><div><b>{title}</b><small>{text}</small></div><label className="switch"><input type="checkbox" defaultChecked={on}/><span/></label></div>}
function Status({n,status,t}:{n:string;status:string;t:Copy}){const label=status==='healthy'?t.healthOperational:status==='degraded'?t.healthDegraded:t.healthOffline;return <div className="setting-line"><div><b>{n}</b><small>{label}</small></div><span className={status==='healthy'?'healthy':'stale'}>● {status.toUpperCase()}</span></div>}
