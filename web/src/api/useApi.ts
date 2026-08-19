import{useCallback,useEffect,useState}from'react';import type{DependencyList}from'react';

/// Generic real-data fetch/poll hook shared by pages that read from the
/// product API, so "unavailable" is always a real fetch failure, never
/// mock data standing in while a request never happened.
export function useApi<T>(fetcher:()=>Promise<T>,deps:DependencyList,pollMs?:number){
  const[data,setData]=useState<T|null>(null);
  const[loading,setLoading]=useState(true);
  const[error,setError]=useState<string|null>(null);
  const load=useCallback(async()=>{
    try{
      const next=await fetcher();
      setData(next);
      setError(null);
    }catch(e){
      setError(e instanceof Error?e.message:'unavailable');
    }finally{
      setLoading(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  },deps);
  useEffect(()=>{
    load();
    if(!pollMs)return;
    const id=window.setInterval(load,pollMs);
    return()=>window.clearInterval(id);
  },[load,pollMs]);
  return{data,loading,error,refresh:load};
}
