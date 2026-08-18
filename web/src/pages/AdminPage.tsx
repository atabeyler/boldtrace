import {useEffect,useState} from 'react';import {useI18n} from '../i18n';import {api} from '../api/client';import type {PendingUser} from '../api/contracts';import {countryName} from '../countries';

export function AdminPage(){
  const{t,lang}=useI18n();
  const[users,setUsers]=useState<PendingUser[]|null>(null);
  const[busyId,setBusyId]=useState<string|null>(null);

  const load=()=>{api.adminPending().then(setUsers).catch(()=>setUsers([]))};
  useEffect(load,[]);

  const act=async(id:string,action:'approve'|'reject')=>{
    setBusyId(id);
    try{
      await(action==='approve'?api.adminApprove(id):api.adminReject(id));
      setUsers(prev=>prev?.filter(u=>u.id!==id)??null);
    }catch{
      /* leave the row in place so the admin can retry */
    }finally{
      setBusyId(null);
    }
  };

  return <div className="page"><div className="page-head"><div><span className="eyebrow">BOLDTRACE / ADMIN</span><h1>{t.adminTitle}</h1></div></div><section className="panel">{users===null?null:users.length===0?<p>{t.adminEmpty}</p>:<div className="admin-table"><div className="admin-row admin-head"><span>{t.adminColName}</span><span>{t.adminColEmail}</span><span>{t.adminColCode}</span><span>{t.adminColCountry}</span><span>{t.adminColId}</span><span>{t.adminColDate}</span><span/></div>{users.map(u=><div className="admin-row" key={u.id}><span>{u.firstName} {u.lastName}</span><span>{u.email}</span><span>{u.userCode}</span><span>{countryName(u.country,lang)}</span><span>{u.nationalId}</span><span>{new Date(u.createdAt).toLocaleDateString(lang)}</span><span className="admin-actions"><button className="primary-small" disabled={busyId===u.id} onClick={()=>act(u.id,'approve')}>{t.adminApprove}</button><button className="primary-small danger" disabled={busyId===u.id} onClick={()=>act(u.id,'reject')}>{t.adminReject}</button></span></div>)}</div>}</section></div>;
}
