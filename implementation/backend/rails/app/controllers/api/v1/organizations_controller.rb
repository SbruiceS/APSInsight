# frozen_string_literal: true

module Api
  module V1
    class OrganizationsController < ApplicationController
      def index
        organizations = Organization.all.order(:name)
        render json: organizations
      end

      def show
        organization = Organization.find(params[:id])
        render json: organization
      end

      def create
        organization = Organization.new(organization_params)
        if organization.save
          AuditEvent.create!(action: "create", resource_type: "Organization", resource_id: organization.id, metadata: { name: organization.name })
          render json: organization, status: :created
        else
          render json: { errors: organization.errors.full_messages }, status: :unprocessable_entity
        end
      end

      def update
        organization = Organization.find(params[:id])
        if organization.update(organization_params)
          render json: organization
        else
          render json: { errors: organization.errors.full_messages }, status: :unprocessable_entity
        end
      end

      private

      def organization_params
        params.require(:organization).permit(:name, :slug, :description)
      end
    end
  end
end
