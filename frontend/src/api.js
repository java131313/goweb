import axios from 'axios'

// 生产模式由 Rocket 同源托管，baseURL 为空即可；
// 需要指向其他后端时，构建前设置 VITE_API_BASE
const api = axios.create({
  baseURL: import.meta.env.VITE_API_BASE || '',
})

export default api
