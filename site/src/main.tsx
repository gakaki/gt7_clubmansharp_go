import React, { lazy, Suspense } from 'react'
import ReactDOM from 'react-dom/client'
import './style.css'

const path = location.pathname.replace(/\/+$/, '') || '/'
const MenuBooksApp = lazy(() => import('./menus/MenuBooksApp'))
const App = lazy(() => import('./App'))

if (path === '/menus_cafe') {
  document.title = 'GT7 菜单书档案 / Café Menu Books'
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <Suspense fallback={<div className="boot-screen">Loading GT7 archive…</div>}>
      {path === '/menus_cafe' ? <MenuBooksApp /> : <App />}
    </Suspense>
  </React.StrictMode>,
)
