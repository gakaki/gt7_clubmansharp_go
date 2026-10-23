const path = location.pathname.replace(/\/+$/, '') || '/'

if (path === '/menus_cafe') {
  document.title = 'GT7 菜单书档案 / Café Menu Books'
  import('./menus/main.jsx')
} else {
  Promise.all([import('vue'), import('./App.vue'), import('./style.css')])
    .then(([{ createApp }, { default: App }]) => {
      createApp(App).mount('#app')
    })
}
