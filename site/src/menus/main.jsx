import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import MenuBooksApp from './MenuBooksApp'

createRoot(document.getElementById('app')).render(
  <StrictMode>
    <MenuBooksApp />
  </StrictMode>,
)
