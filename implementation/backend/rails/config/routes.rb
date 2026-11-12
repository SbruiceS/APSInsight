Rails.application.routes.draw do
  namespace :api do
    namespace :v1 do
      get "health/live", to: "health#live"
      get "health/ready", to: "health#ready"
      get "capabilities", to: "health#capabilities"

      resources :organizations, only: [:index, :show, :create, :update]
      resources :projects, only: [:index, :show, :create, :update]
      resources :datasets, only: [:index, :show, :create, :update] do
        member do
          get :schema
          get :preview
          get "analytics/soil_summary", to: "analytics#soil_summary"
        end
      end
      resources :imports, only: [:create, :show]

      # Identity
      resources :users, only: [:index, :show, :create, :update]
      resources :roles, only: [:index, :show, :create]
      resources :memberships, only: [:index, :show, :create, :destroy]

      # Agricultural domain
      resources :farms, only: [:index, :show, :create, :update]
      resources :fields, only: [:index, :show, :create]
      resources :plots, only: [:index, :show, :create]
      resources :soil_samples, only: [:index, :show, :create]
      resources :observations, only: [:index, :show, :create]

      # Schema & quality
      resources :dataset_schemas, only: [:index, :show, :create]
      resources :data_quality_reports, only: [:index, :show, :create]

      # AgriQL query language
      post "agriql/query", to: "agriql#query"
      get "agriql/capabilities", to: "agriql#capabilities"
      get "agriql/schema", to: "agriql#schema_discovery"

      # AgriCUDa kernels
      get "agricuda/capabilities", to: "agricuda#capabilities"
      post "agricuda/run", to: "agricuda#run"
      post "agricuda/check", to: "agricuda#check"
    end
  end
end