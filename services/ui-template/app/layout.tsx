import React from "react"
import type { Metadata, Viewport } from "next"
import { Inter, JetBrains_Mono } from "next/font/google"
import { ThemeProvider } from "@/components/providers/theme-provider"
import { Toaster } from "sonner"
import { product } from "@/lib/product"

import "./globals.css"

const inter = Inter({ subsets: ["latin"], variable: "--font-inter" })
const jetbrains = JetBrains_Mono({ subsets: ["latin"], variable: "--font-jetbrains" })

export const metadata: Metadata = {
  title: {
    default: product.title,
    template: `%s | ${product.name}`,
  },
  description: product.description,
  keywords: [
    "verificacao criptografica",
    "SIRP",
    "compliance",
    "auditoria",
    "bundle offline",
    "prova criptografica",
  ],
  openGraph: {
    title: product.title,
    description: product.description,
    type: "website",
    locale: product.locale.replace("-", "_"),
  },
}

export const viewport: Viewport = {
  themeColor: product.theme.primary,
  width: "device-width",
  initialScale: 1,
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang={product.locale} suppressHydrationWarning>
      <body className={`${inter.variable} ${jetbrains.variable} font-sans antialiased`}>
        <ThemeProvider
          attribute="class"
          defaultTheme="light"
          enableSystem
          disableTransitionOnChange
        >
          {children}
          <Toaster position="top-right" />
        </ThemeProvider>
      </body>
    </html>
  )
}
