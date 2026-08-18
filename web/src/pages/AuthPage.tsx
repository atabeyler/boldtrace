import {useState} from 'react';import {Brand} from '../components/Brand';import {LanguageSelector} from '../components/LanguageSelector';import {Footer} from '../components/Footer';import {useI18n} from '../i18n';import {api,ApiError} from '../api/client';import type {Account} from '../api/contracts';import {sortedCountries} from '../countries';

const ERROR_KEYS:Record<string,'errorInvalidInput'|'errorTermsRequired'|'errorEmailTaken'|'errorInvalidCredentials'|'errorServiceUnavailable'|'errorRateLimited'|'errorInvalidUserCode'|'errorUserCodeTaken'|'errorAccountPending'|'errorAccountRejected'>={
  invalid_input:'errorInvalidInput',
  terms_not_accepted:'errorTermsRequired',
  email_taken:'errorEmailTaken',
  invalid_credentials:'errorInvalidCredentials',
  database_unavailable:'errorServiceUnavailable',
  rate_limited:'errorRateLimited',
  invalid_user_code:'errorInvalidUserCode',
  user_code_taken:'errorUserCodeTaken',
  account_pending:'errorAccountPending',
  account_rejected:'errorAccountRejected',
};

export function AuthPage({onAuthenticated}:{onAuthenticated:(account:Account)=>void}){
  const{t,lang}=useI18n();
  const[mode,setMode]=useState<'login'|'register'>('login');
  const[busy,setBusy]=useState(false);
  const[error,setError]=useState<string|null>(null);
  const[pending,setPending]=useState(false);
  const countries=sortedCountries(lang);

  const submit=async(e:React.FormEvent<HTMLFormElement>)=>{
    e.preventDefault();
    setError(null);
    setBusy(true);
    const form=new FormData(e.currentTarget);
    try{
      if(mode==='login'){
        const account=await api.login({
          email:String(form.get('email')||''),
          password:String(form.get('password')||''),
          rememberMe:form.get('rememberMe')==='on',
        });
        onAuthenticated(account);
      }else{
        const account=await api.register({
          firstName:String(form.get('firstName')||''),
          lastName:String(form.get('lastName')||''),
          userCode:String(form.get('userCode')||''),
          country:String(form.get('country')||''),
          nationalId:String(form.get('nationalId')||''),
          email:String(form.get('email')||''),
          password:String(form.get('password')||''),
          language:lang,
          termsAccepted:form.get('termsAccepted')==='on',
        });
        if(account.status==='approved'){
          onAuthenticated(account);
        }else{
          setPending(true);
          setMode('login');
        }
      }
    }catch(err){
      const code=err instanceof ApiError?err.code:'errorGeneric';
      setError(t[ERROR_KEYS[code]??'errorGeneric']);
    }finally{
      setBusy(false);
    }
  };

  return <div className="auth-page"><div className="auth-grid"/><header className="auth-header"><Brand/><LanguageSelector/></header><section className="auth-hero"><div className="eyebrow"><span/>BOLDTRACE / INTELLIGENCE ACCESS</div><h1>{t.welcome}</h1><p>{t.subtitle}</p><div className="boot-strip"><span>MARKET FEEDS <b>ONLINE</b></span><span>RISK GUARDIAN <b>ACTIVE</b></span><span>ADAPTIVE LEARNING <b>READY</b></span></div></section><section className="auth-panel"><div className="auth-card"><div className="auth-tabs"><button type="button" className={mode==='login'?'active':''} onClick={()=>{setMode('login');setError(null);setPending(false)}}>{t.login}</button><button type="button" className={mode==='register'?'active':''} onClick={()=>{setMode('register');setError(null);setPending(false)}}>{t.register}</button></div>{pending&&<div className="auth-pending" role="status"><b>{t.pendingApprovalTitle}</b><p>{t.pendingApprovalBody}</p></div>}<form onSubmit={submit}>{mode==='register'&&<><label>{t.firstName}<input required name="firstName" autoComplete="given-name" placeholder={t.firstName}/></label><label>{t.lastName}<input required name="lastName" autoComplete="family-name" placeholder={t.lastName}/></label><label>{t.userCode}<input required name="userCode" minLength={4} maxLength={20} pattern="[A-Za-z0-9]{4,20}" placeholder={t.userCode} title={t.userCodeHint}/></label><label>{t.country}<select required name="country" defaultValue=""><option value="" disabled>{t.country}</option>{countries.map(c=><option key={c.code} value={c.code}>{c.name}</option>)}</select></label><label>{t.nationalId}<input required name="nationalId" placeholder={t.nationalId} title={t.nationalIdHint}/></label></>}<label>{t.email}<input required name="email" type="email" autoComplete="email" placeholder="operator@boldtrace.ai"/></label><label>{t.password}<input required name="password" type="password" minLength={8} autoComplete={mode==='login'?'current-password':'new-password'} placeholder="••••••••••••"/></label>{mode==='login'&&<div className="auth-row"><label className="check"><input type="checkbox" name="rememberMe"/> {t.rememberMe}</label></div>}{mode==='register'&&<label className="check terms-check"><input required type="checkbox" name="termsAccepted"/> {t.termsCheckbox}</label>}{error&&<p className="auth-error" role="alert">{error}</p>}<button className="primary-action" disabled={busy}>{busy?t.authorizing:mode==='login'?t.login:t.register}</button></form><div className="security-note"><span>◇</span><p><b>Encrypted access</b><br/>Session and account controls are protected by the BOLDTRACE security layer.</p></div></div></section><Footer/></div>;
}
