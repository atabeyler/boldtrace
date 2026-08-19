import {useI18n} from '../i18n';import {Brand} from './Brand';
export function MenuPanel({onNavigate,onClose}:{onNavigate?:(id:string)=>void;onClose:()=>void}){
  const{t}=useI18n();
  const go=(id:string)=>{onNavigate?.(id);onClose()};
  return <div className="popover menu-panel">
    <div className="menu-panel-brand"><Brand compact/><small>{t.footer}</small></div>
    {onNavigate&&<div className="menu-panel-links">
      <button onClick={()=>go('settings')}>{t.navSettings}</button>
      <button onClick={()=>go('health')}>{t.navHealth}</button>
    </div>}
    <div className="menu-panel-foot">
      <p>{t.footer} © {new Date().getFullYear()} BOLD</p>
      <p>{t.disclaimer}</p>
    </div>
  </div>;
}
