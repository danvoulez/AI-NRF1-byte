import productJson from "../product.json"

export interface ProductConfig {
  name: string
  slug: string
  registry: string
  tenant: string
  theme: {
    primary: string
    accent: string
    radius: string
  }
  logo: string
  title: string
  description: string
  locale: string
  pages: string[]
  features: {
    run_pipeline: boolean
    team_management: boolean
    billing: boolean
    marketing_pages: boolean
  }
}

export const product: ProductConfig = productJson as ProductConfig

export function hasPage(page: string): boolean {
  return product.pages.includes(page)
}

export function hasFeature(feature: keyof ProductConfig["features"]): boolean {
  return product.features[feature] === true
}
