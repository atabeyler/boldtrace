import {useI18n,type Lang} from '../i18n';
const langs:[Lang,string][]=[['tr','Türkçe'],['en','English'],['de','Deutsch'],['fr','Français'],['ar','العربية'],['ru','Русский']];
export function SettingsPanel({onOpenSettings,onClose}:{onOpenSettings:()=>void;onClose:()=>void}){
  const{t,lang,setLang}=useI18n();
  return <div className="popover settings-panel-dropdown">
    <span className="eyebrow">{t.settingsLanguage}</span>
    <div className="settings-panel-langs">
      {langs.map(([code,label])=><button key={code} className={lang===code?'active':''} dir={code==='ar'?'rtl':'ltr'} onClick={()=>setLang(code)}>
        <span>{label}</span>{lang===code&&<span aria-hidden="true">✓</span>}
      </button>)}
    </div>
    <button className="settings-panel-full" onClick={()=>{onOpenSettings();onClose()}}>{t.openFullSettings}</button>
  </div>;
}
