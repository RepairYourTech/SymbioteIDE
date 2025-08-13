# Symbiote Website Plan

## Overview
Marketing website and user portal for Symbiote. Handles signups, subscriptions, user management, and serves as the main web presence.

## Technology Stack
- **Framework**: Next.js 14 (App Router)
- **Language**: TypeScript
- **Styling**: Tailwind CSS
- **Database**: Firebase Firestore
- **Authentication**: Firebase Auth
- **Payments**: Stripe
- **Hosting**: Firebase App Hosting
- **Analytics**: Firebase Analytics

## Key Features

### Marketing Site
- Landing page with product showcase
- Pricing plans and subscription tiers
- Feature demonstrations
- Blog/documentation
- Contact and support

### User Portal
- User registration and login
- Subscription management
- Billing and payment history
- Account settings
- Download links for desktop app
- Usage analytics and limits

### Admin Dashboard
- User management
- Subscription analytics
- Support ticket system
- Feature flag management
- System monitoring

## Pages Structure
```
/                     # Landing page
/pricing             # Pricing plans
/features            # Feature showcase
/docs               # Documentation
/blog               # Blog/updates
/login              # User login
/signup             # User registration
/dashboard          # User portal
/admin              # Admin dashboard
/api                # API routes
```

## Database Schema (Firebase)
- users collection
- subscriptions collection
- payments collection
- support_tickets collection
- feature_flags collection
- analytics_events collection

## Subscription Tiers
- **Free**: Basic features, limited usage
- **Pro**: Full features, higher limits
- **Team**: Multi-user, collaboration features
- **Enterprise**: Custom limits, priority support

## Integration Points
- Firebase Auth for authentication
- Stripe for payment processing
- Firebase Firestore for data storage
- Shared components from design system
- API client for backend communication

## Build & Deploy
- GitHub Actions for CI/CD
- Firebase App Hosting for deployment
- Automatic deployments on main branch
- Preview deployments for PRs
